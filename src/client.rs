use std::{collections::HashMap, marker::PhantomData, pin::Pin, sync::Arc};

use async_executors::{JoinHandle, SpawnHandle, SpawnHandleExt};
use futures::{
    channel::{mpsc, oneshot},
    lock::Mutex,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
    task::{Context, Poll},
};
use serde::Serialize;
use uuid::Uuid;

use super::{
    graphql::{self, GraphqlOperation},
    protocol::{ConnectionAck, ConnectionInit, Event, Message},
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
    phantom: PhantomData<*const GraphqlClient>,
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
        runtime: impl SpawnHandle<Result<(), Error>>,
    ) -> Result<AsyncWebsocketClient<GraphqlClient, WsMessage>, Error>
    where
        GraphqlClient: crate::graphql::GraphqlClient + Send + 'static,
        WsMessage: WebsocketMessage + Send + 'static,
    {
        websocket_sink
            .send(json_message(ConnectionInit::new(self.payload))?)
            .await
            .map_err(|err| Error::Send(err.to_string()))?;

        match websocket_stream.next().await {
            None => todo!(),
            Some(Err(_)) => todo!(),
            Some(Ok(data)) => {
                decode_message::<ConnectionAck<()>, _>(data)?;
            }
        }

        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

        let operations = Arc::new(Mutex::new(HashMap::new()));

        let receiver_handle = runtime
            .spawn_handle(receiver_loop::<_, _, GraphqlClient>(
                websocket_stream,
                Arc::clone(&operations),
                shutdown_sender,
            ))
            .map_err(|err| Error::SpawnHandle(err.to_string()))?;

        let (sender_sink, sender_stream) = mpsc::channel(1);

        let sender_handle = runtime
            .spawn_handle(sender_loop(
                sender_stream,
                websocket_sink,
                Arc::clone(&operations),
                shutdown_receiver,
            ))
            .map_err(|err| Error::SpawnHandle(err.to_string()))?;

        Ok(AsyncWebsocketClient {
            inner: Arc::new(ClientInner {
                receiver_handle,
                operations,
                sender_handle,
            }),
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
    ) -> Result<SubscriptionStream<GraphqlClient, Operation>, Error>
    where
        Operation:
            GraphqlOperation<GenericResponse = GraphqlClient::Response> + Unpin + Send + 'static,
    {
        let id = Uuid::new_v4();
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

        Ok(SubscriptionStream::<GraphqlClient, Operation> {
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
            phantom: PhantomData,
        })
    }
}

/// A stream of GraphQL subscription updates.
#[pin_project::pin_project]
pub struct SubscriptionStream<GraphqlClient, Operation>
where
    GraphqlClient: graphql::GraphqlClient,
    Operation: GraphqlOperation<GenericResponse = GraphqlClient::Response>,
{
    id: String,
    stream: Pin<Box<dyn Stream<Item = Result<Operation::Response, Error>> + Send>>,
    cancel_func: Box<dyn FnOnce() -> futures::future::BoxFuture<'static, Result<(), Error>> + Send>,
    phantom: PhantomData<GraphqlClient>,
}

impl<GraphqlClient, Operation> SubscriptionStream<GraphqlClient, Operation>
where
    GraphqlClient: graphql::GraphqlClient + Send,
    Operation: GraphqlOperation<GenericResponse = GraphqlClient::Response> + Send,
{
    /// Cancels the subscription.
    pub async fn stop_operation(self) -> Result<(), Error> {
        (self.cancel_func)().await
    }
}

impl<GraphqlClient, Operation> Stream for SubscriptionStream<GraphqlClient, Operation>
where
    GraphqlClient: graphql::GraphqlClient,
    Operation: GraphqlOperation<GenericResponse = GraphqlClient::Response> + Unpin,
{
    type Item = Result<Operation::Response, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.as_mut().poll_next(cx)
    }
}

type OperationSender<GenericResponse> = mpsc::Sender<GenericResponse>;

type OperationMap<GenericResponse> = Arc<Mutex<HashMap<Uuid, OperationSender<GenericResponse>>>>;

async fn receiver_loop<S, WsMessage, GraphqlClient>(
    mut receiver: S,
    operations: OperationMap<GraphqlClient::Response>,
    shutdown: oneshot::Sender<()>,
) -> Result<(), Error>
where
    S: Stream<Item = Result<WsMessage, WsMessage::Error>> + Unpin,
    WsMessage: WebsocketMessage,
    GraphqlClient: crate::graphql::GraphqlClient,
{
    while let Some(msg) = receiver.next().await {
        println!("Received message: {:?}", msg);
        if handle_message::<WsMessage, GraphqlClient>(msg, &operations)
            .await
            .is_err()
        {
            println!("Error happened, killing things");
            break;
        }
    }

    shutdown
        .send(())
        .map_err(|_| Error::SenderShutdown("Couldn't shutdown sender".to_owned()))
}

async fn handle_message<WsMessage, GraphqlClient>(
    msg: Result<WsMessage, WsMessage::Error>,
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

    let id = &Uuid::parse_str(event.id()).map_err(|err| Error::Decode(err.to_string()))?;
    match event {
        Event::Next { payload, .. } => {
            let mut sink = operations
                .lock()
                .await
                .get(&id)
                .ok_or_else(|| {
                    Error::Decode("Received message for unknown subscription".to_owned())
                })?
                .clone();

            sink.send(payload)
                .await
                .map_err(|err| Error::Send(err.to_string()))?
        }
        Event::Complete { .. } => {
            println!("Stream complete");
            operations.lock().await.remove(&id);
        }
        Event::Error { payload, .. } => {
            let mut sink = operations.lock().await.remove(&id).ok_or_else(|| {
                Error::Decode("Received error for unknown subscription".to_owned())
            })?;

            sink.send(
                GraphqlClient::error_response(payload)
                    .map_err(|err| Error::Send(err.to_string()))?,
            )
            .await
            .map_err(|err| Error::Send(err.to_string()))?;
        }
    }

    Ok(())
}

async fn sender_loop<M, S, E, GenericResponse>(
    message_stream: mpsc::Receiver<M>,
    mut ws_sender: S,
    operations: OperationMap<GenericResponse>,
    shutdown: oneshot::Receiver<()>,
) -> Result<(), Error>
where
    M: WebsocketMessage,
    S: Sink<M, Error = E> + Unpin,
    E: std::error::Error,
{
    use futures::{future::FutureExt, select};

    let mut message_stream = message_stream.fuse();
    let mut shutdown = shutdown.fuse();

    loop {
        select! {
            msg = message_stream.next() => {
                if let Some(msg) = msg {
                    println!("Sending message: {:?}", msg);
                    ws_sender
                        .send(msg)
                        .await
                        .map_err(|err| Error::Send(err.to_string()))?;
                } else {
                    return Ok(());
                }
            }
            _ = shutdown => {
                // Shutdown the incoming message stream
                let mut message_stream = message_stream.into_inner();
                message_stream.close();
                while message_stream.next().await.is_some() {}

                // Clear out any operations
                operations.lock().await.clear();

                return Ok(());
            }
        }
    }
}

struct ClientInner<GraphqlClient>
where
    GraphqlClient: crate::graphql::GraphqlClient,
{
    #[allow(dead_code)]
    receiver_handle: JoinHandle<Result<(), Error>>,
    #[allow(dead_code)]
    sender_handle: JoinHandle<Result<(), Error>>,
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
        Err(Error::Close(
            message.error_message().unwrap_or(String::new()).to_owned(),
        ))
    } else if let Some(s) = message.text() {
        println!("Received {}", s);
        Ok(Some(
            serde_json::from_str::<T>(&s).map_err(|err| Error::Decode(err.to_string()))?,
        ))
    } else {
        Ok(None)
    }
}
