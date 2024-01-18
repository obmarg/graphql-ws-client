use std::{
    collections::HashMap,
    marker::PhantomData,
    pin::Pin,
    sync::{
        atomic::{self, AtomicU64},
        Arc,
    },
};

use futures::{
    channel::mpsc,
    future::RemoteHandle,
    lock::Mutex,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
    task::{Context, Poll, SpawnExt},
};
use serde::Serialize;

use super::{
    graphql::{self, GraphqlOperation},
    logging::trace,
    protocol::{ConnectionInit, Event, Message},
    websockets::WebsocketMessage,
};

const SUBSCRIPTION_BUFFER_SIZE: usize = 5;

/// A websocket client
pub struct AsyncWebsocketClient<GraphqlClient, WsMessage>
where
    GraphqlClient: graphql::GraphqlClient,
{
    inner: Arc<ClientInner<GraphqlClient>>,
    sender_sink: mpsc::Sender<WsMessage>,
    next_id: AtomicU64,
    phantom: PhantomData<GraphqlClient>,
}

#[derive(thiserror::Error, Debug)]
/// Error type
pub enum Error {
    /// Unknown error
    #[error("unknown: {0}")]
    Unknown(String),
    /// Custom error
    #[error("{0}: {1}")]
    Custom(String, String),
    /// Unexpected close frame
    #[error("got close frame, reason: {0}")]
    Close(String),
    /// Decoding / parsing error
    #[error("message decode error, reason: {0}")]
    Decode(String),
    /// Sending error
    #[error("message sending error, reason: {0}")]
    Send(String),
    /// Futures spawn error
    #[error("futures spawn error, reason: {0}")]
    SpawnHandle(String),
    /// Sender shutdown error
    #[error("sender shutdown error, reason: {0}")]
    SenderShutdown(String),
}

#[derive(Serialize)]
pub enum NoPayload {}

/// A websocket client builder
pub struct AsyncWebsocketClientBuilder<GraphqlClient, Payload = NoPayload>
where
    GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
{
    payload: Option<Payload>,
    phantom: PhantomData<fn() -> GraphqlClient>,
}

impl<GraphqlClient, Payload> AsyncWebsocketClientBuilder<GraphqlClient, Payload>
where
    GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
{
    /// Constructs an AsyncWebsocketClientBuilder
    pub fn new() -> Self {
        Self {
            payload: None,
            phantom: PhantomData,
        }
    }

    /// Add payload to `connection_init`
    pub fn payload<NewPayload: Serialize>(
        self,
        payload: NewPayload,
    ) -> AsyncWebsocketClientBuilder<GraphqlClient, NewPayload> {
        AsyncWebsocketClientBuilder {
            payload: Some(payload),
            phantom: PhantomData,
        }
    }
}

impl<GraphqlClient, Payload> Default for AsyncWebsocketClientBuilder<GraphqlClient, Payload>
where
    GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<GraphqlClient, Payload> AsyncWebsocketClientBuilder<GraphqlClient, Payload>
where
    GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
    Payload: Serialize,
{
    /// Constructs an AsyncWebsocketClient
    ///
    /// Accepts a stream and a sink for the underlying websocket connection,
    /// and an `async_executors::SpawnHandle` that tells the client which
    /// async runtime to use.
    pub async fn build<WsMessage>(
        self,
        mut websocket_stream: impl Stream<Item = Result<WsMessage, WsMessage::Error>>
            + Unpin
            + Send
            + 'static,
        mut websocket_sink: impl Sink<WsMessage, Error = WsMessage::Error> + Unpin + Send + 'static,
        runtime: impl SpawnExt,
    ) -> Result<AsyncWebsocketClient<GraphqlClient, WsMessage>, Error>
    where
        GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
        WsMessage: WebsocketMessage + Send + 'static,
    {
        websocket_sink
            .send(json_message(ConnectionInit::new(self.payload))?)
            .await
            .map_err(|err| Error::Send(err.to_string()))?;

        let operations = Arc::new(Mutex::new(HashMap::new()));

        let (mut sender_sink, sender_stream) = mpsc::channel(1);

        let sender_handle = runtime
            .spawn_with_handle(async move {
                sender_stream
                    .map(Ok)
                    .forward(websocket_sink)
                    .await
                    .map_err(|error| Error::Send(error.to_string()))
            })
            .map_err(|err| Error::SpawnHandle(err.to_string()))?;

        // wait for ack before entering receiver loop:
        loop {
            match websocket_stream.next().await {
                None => todo!(),
                Some(msg) => {
                    let event = decode_message::<Event<GraphqlClient::Response>, WsMessage>(
                        msg.map_err(|err| Error::Decode(err.to_string()))?,
                    )
                    .map_err(|err| Error::Decode(err.to_string()))?;
                    match event {
                        // pings can be sent at any time
                        Some(Event::Ping { .. }) => {
                            let msg = json_message(Message::<()>::Pong)
                                .map_err(|err| Error::Send(err.to_string()))?;
                            sender_sink
                                .send(msg)
                                .await
                                .map_err(|err| Error::Send(err.to_string()))?;
                        }
                        Some(Event::ConnectionAck { .. }) => {
                            // handshake completed, ready to enter main receiver loop
                            trace!("connection_ack received, handshake completed");
                            break;
                        }
                        Some(event) => {
                            return Err(Error::Decode(format!(
                                "expected a connection_ack or ping, got {}",
                                event.r#type()
                            )));
                        }
                        None => {}
                    }
                }
            }
        }

        let receiver_handle = runtime
            .spawn_with_handle(receiver_loop::<_, _, GraphqlClient>(
                websocket_stream,
                sender_sink.clone(),
                Arc::clone(&operations),
            ))
            .map_err(|err| Error::SpawnHandle(err.to_string()))?;

        Ok(AsyncWebsocketClient {
            inner: Arc::new(ClientInner {
                receiver_handle,
                operations,
                sender_handle,
            }),
            next_id: 0.into(),
            sender_sink,
            phantom: PhantomData,
        })
    }
}

impl<GraphqlClient, WsMessage> AsyncWebsocketClient<GraphqlClient, WsMessage>
where
    WsMessage: WebsocketMessage + Send + 'static,
    GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
{
    /*
    pub async fn operation<'a, T: 'a>(&self, _op: Operation<'a, T>) -> Result<(), ()> {
        todo!()
        // Probably hook into streaming operation and do a take 1 -> into_future
    }*/

    /// Starts a streaming operation on this client.
    ///
    /// Returns a `Stream` of responses.
    pub async fn streaming_operation<'a, Operation>(
        &mut self,
        op: Operation,
    ) -> Result<SubscriptionStream<Operation>, Error>
    where
        Operation:
            GraphqlOperation<GenericResponse = GraphqlClient::Response> + Unpin + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, atomic::Ordering::Relaxed);
        let (sender, receiver) = mpsc::channel(SUBSCRIPTION_BUFFER_SIZE);

        self.inner.operations.lock().await.insert(id, sender);

        let msg = json_message(Message::Subscribe {
            id: id.to_string(),
            payload: &op,
        })
        .map_err(|err| Error::Decode(err.to_string()))?;

        self.sender_sink
            .send(msg)
            .await
            .map_err(|err| Error::Send(err.to_string()))?;

        let mut sender_clone = self.sender_sink.clone();
        let id_clone = id.to_string();

        Ok(SubscriptionStream::<Operation> {
            id: id.to_string(),
            stream: Box::pin(receiver.map(move |response| {
                op.decode(response)
                    .map_err(|err| Error::Decode(err.to_string()))
            })),
            cancel_func: Box::new(move || {
                Box::pin(async move {
                    let msg: Message<()> = Message::Complete { id: id_clone };

                    sender_clone
                        .send(json_message(msg)?)
                        .await
                        .map_err(|err| Error::Send(err.to_string()))?;

                    Ok(())
                })
            }),
        })
    }
}

/// A `futures::Stream` for a subscription.
///
/// Emits an item for each message received by the subscription.
#[pin_project::pin_project]
pub struct SubscriptionStream<Operation>
where
    Operation: GraphqlOperation,
{
    id: String,
    stream: Pin<Box<dyn Stream<Item = Result<Operation::Response, Error>> + Send>>,
    cancel_func: Box<dyn FnOnce() -> futures::future::BoxFuture<'static, Result<(), Error>> + Send>,
}

impl<Operation> SubscriptionStream<Operation>
where
    Operation: GraphqlOperation + Send,
{
    /// Stops the operation by sending a Complete message to the server.
    pub async fn stop_operation(self) -> Result<(), Error> {
        (self.cancel_func)().await
    }
}

impl<Operation> Stream for SubscriptionStream<Operation>
where
    Operation: GraphqlOperation + Unpin,
{
    type Item = Result<Operation::Response, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.as_mut().poll_next(cx)
    }
}

type OperationSender<GenericResponse> = mpsc::Sender<GenericResponse>;

type OperationMap<GenericResponse> = Arc<Mutex<HashMap<u64, OperationSender<GenericResponse>>>>;

async fn receiver_loop<S, WsMessage, GraphqlClient>(
    mut receiver: S,
    mut sender: mpsc::Sender<WsMessage>,
    operations: OperationMap<GraphqlClient::Response>,
) -> Result<(), Error>
where
    S: Stream<Item = Result<WsMessage, WsMessage::Error>> + Unpin,
    WsMessage: WebsocketMessage,
    GraphqlClient: crate::graphql::GraphqlClient,
{
    while let Some(msg) = receiver.next().await {
        trace!("Received message: {:?}", msg);
        if let Err(err) =
            handle_message::<WsMessage, GraphqlClient>(msg, &mut sender, &operations).await
        {
            trace!("message handler error, shutting down: {err:?}");
            #[cfg(feature = "no-logging")]
            let _ = err;
            break;
        }
    }

    // Clear out any operations
    operations.lock().await.clear();

    Ok(())
}

async fn handle_message<WsMessage, GraphqlClient>(
    msg: Result<WsMessage, WsMessage::Error>,
    sender: &mut mpsc::Sender<WsMessage>,
    operations: &OperationMap<GraphqlClient::Response>,
) -> Result<(), Error>
where
    WsMessage: WebsocketMessage,
    GraphqlClient: crate::graphql::GraphqlClient,
{
    let event = decode_message::<Event<GraphqlClient::Response>, WsMessage>(
        msg.map_err(|err| Error::Decode(err.to_string()))?,
    )
    .map_err(|err| Error::Decode(err.to_string()))?;

    let event = match event {
        Some(event) => event,
        None => return Ok(()),
    };

    let id = match event.id() {
        Some(id) => Some(
            id.parse::<u64>()
                .map_err(|err| Error::Decode(err.to_string()))?,
        ),
        None => None,
    };

    match event {
        Event::Next { payload, .. } => {
            let mut sink = operations
                .lock()
                .await
                .get(id.as_ref().expect("id for next event"))
                .ok_or_else(|| {
                    Error::Decode("Received message for unknown subscription".to_owned())
                })?
                .clone();

            sink.send(payload)
                .await
                .map_err(|err| Error::Send(err.to_string()))?
        }
        Event::Complete { .. } => {
            trace!("Stream complete");
            operations
                .lock()
                .await
                .remove(id.as_ref().expect("id for complete event"));
        }
        Event::Error { payload, .. } => {
            let mut sink = operations
                .lock()
                .await
                .remove(id.as_ref().expect("id for error event"))
                .ok_or_else(|| {
                    Error::Decode("Received error for unknown subscription".to_owned())
                })?;

            sink.send(
                GraphqlClient::error_response(payload)
                    .map_err(|err| Error::Send(err.to_string()))?,
            )
            .await
            .map_err(|err| Error::Send(err.to_string()))?;
        }
        Event::ConnectionAck { .. } => {
            return Err(Error::Decode("unexpected connection_ack".to_string()))
        }
        Event::Ping { .. } => {
            let msg =
                json_message(Message::<()>::Pong).map_err(|err| Error::Send(err.to_string()))?;
            sender
                .send(msg)
                .await
                .map_err(|err| Error::Send(err.to_string()))?;
        }
        Event::Pong { .. } => {}
    }

    Ok(())
}

struct ClientInner<GraphqlClient>
where
    GraphqlClient: crate::graphql::GraphqlClient,
{
    #[allow(dead_code)]
    receiver_handle: RemoteHandle<Result<(), Error>>,
    #[allow(dead_code)]
    sender_handle: RemoteHandle<Result<(), Error>>,
    operations: OperationMap<GraphqlClient::Response>,
}

fn json_message<M: WebsocketMessage>(payload: impl serde::Serialize) -> Result<M, Error> {
    Ok(M::new(
        serde_json::to_string(&payload).map_err(|err| Error::Decode(err.to_string()))?,
    ))
}

fn decode_message<T: serde::de::DeserializeOwned, WsMessage: WebsocketMessage>(
    message: WsMessage,
) -> Result<Option<T>, Error> {
    if message.is_ping() || message.is_pong() {
        Ok(None)
    } else if message.is_close() {
        Err(Error::Close(message.error_message().unwrap_or_default()))
    } else if let Some(s) = message.text() {
        trace!("Decoding message: {}", s);
        Ok(Some(
            serde_json::from_str::<T>(s).map_err(|err| Error::Decode(err.to_string()))?,
        ))
    } else {
        Ok(None)
    }
}
