use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use async_executors::{JoinHandle, SpawnHandle, SpawnHandleExt};
use futures::{
    channel::{mpsc, oneshot},
    lock::Mutex,
    sink::{Sink, SinkExt},
    stream::{Stream, StreamExt},
};
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
    phantom: PhantomData<*const GraphqlClient>,
}

#[derive(thiserror::Error, Debug)]
#[error("Something went wrong")]
pub struct Error {}

impl<GraphqlClient, WsMessage> AsyncWebsocketClient<GraphqlClient, WsMessage>
where
    WsMessage: WebsocketMessage + Send + 'static,
    GraphqlClient: crate::graphql::GraphqlClient + 'static,
{
    /// Constructs an AsyncWebsocketClient
    ///
    /// Accepts a stream and a sink for the underlying websocket connection,
    /// and an `async_executors::SpawnHandle` that tells the client which
    /// async runtime to use.
    pub async fn new(
        websocket_stream: impl Stream<Item = Result<WsMessage, WsMessage::Error>>
            + Unpin
            + Send
            + 'static,
        websocket_sink: impl Sink<WsMessage, Error = WsMessage::Error> + Unpin + Send + 'static,
        runtime: impl SpawnHandle<()>,
    ) -> Result<Self, Error> {
        Self::new_with_connection_payload(Option::<()>::None, websocket_stream, websocket_sink, runtime).await
    }

    /// Constructs an AsyncWebsocketClient
    ///
    /// Accepts a connection_init payload and a stream and a sink for the underlying websocket connection,
    /// and an `async_executors::SpawnHandle` that tells the client which
    /// async runtime to use.
    pub async fn new_with_connection_payload<Payload>(
        connection_init_payload: Option<Payload>,
        mut websocket_stream: impl Stream<Item = Result<WsMessage, WsMessage::Error>>
            + Unpin
            + Send
            + 'static,
        mut websocket_sink: impl Sink<WsMessage, Error = WsMessage::Error> + Unpin + Send + 'static,
        runtime: impl SpawnHandle<()>,
    ) -> Result<Self, Error>
    where
        Payload: serde::Serialize,
    {
        websocket_sink
            .send(json_message(ConnectionInit::new(connection_init_payload)).unwrap())
            .await
            .map_err(|_| Error {})?;

        match websocket_stream.next().await {
            None => todo!(),
            Some(Err(_)) => todo!(),
            Some(Ok(data)) => {
                decode_message::<ConnectionAck<()>, _>(data).unwrap();
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
            .unwrap();

        let (sender_sink, sender_stream) = mpsc::channel(1);

        let sender_handle = runtime
            .spawn_handle(sender_loop(
                sender_stream,
                websocket_sink,
                Arc::clone(&operations),
                shutdown_receiver,
            ))
            .unwrap();

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

    /*
    pub async fn operation<'a, T: 'a>(&self, _op: Operation<'a, T>) -> Result<(), ()> {
        todo!()
        // Probably hook into streaming operation and do a take 1 -> into_future
    }*/

    /// Starts a streaming operation on this client.
    ///
    /// Returns a `Stream` of responses.
    pub async fn streaming_operation<Operation>(
        &mut self,
        op: Operation,
    ) -> impl Stream<Item = Operation::Response>
    where
        Operation: GraphqlOperation<GenericResponse = GraphqlClient::Response>,
    {
        let id = Uuid::new_v4();
        let (sender, receiver) = mpsc::channel(SUBSCRIPTION_BUFFER_SIZE);

        self.inner.operations.lock().await.insert(id, sender);

        let msg = json_message(Message::Subscribe {
            id: id.to_string(),
            payload: &op,
        })
        .unwrap();

        self.sender_sink.send(msg).await.unwrap();

        receiver.map(move |response| op.decode(response).unwrap())
    }
}

type OperationSender<GenericResponse> = mpsc::Sender<GenericResponse>;

type OperationMap<GenericResponse> = Arc<Mutex<HashMap<Uuid, OperationSender<GenericResponse>>>>;

async fn receiver_loop<S, WsMessage, GraphqlClient>(
    mut receiver: S,
    operations: OperationMap<GraphqlClient::Response>,
    shutdown: oneshot::Sender<()>,
) where
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

    shutdown.send(()).expect("Couldn't shutdown sender");
}

type BoxError = Box<dyn std::error::Error>;

async fn handle_message<WsMessage, GraphqlClient>(
    msg: Result<WsMessage, WsMessage::Error>,
    operations: &OperationMap<GraphqlClient::Response>,
) -> Result<(), BoxError>
where
    WsMessage: WebsocketMessage,
    GraphqlClient: crate::graphql::GraphqlClient,
{
    let event = decode_message::<Event<GraphqlClient::Response>, WsMessage>(msg?)?;

    if event.is_none() {
        return Ok(());
    }
    let event = event.unwrap();

    let id = &Uuid::parse_str(event.id())?;
    match event {
        Event::Next { payload, .. } => {
            let mut sink = operations
                .lock()
                .await
                .get(&id)
                .ok_or("Received message for unknown subscription")?
                .clone();

            sink.send(payload).await?
        }
        Event::Complete { .. } => {
            println!("Stream complete");
            operations.lock().await.remove(&id);
        }
        Event::Error { payload, .. } => {
            let mut sink = operations
                .lock()
                .await
                .remove(&id)
                .ok_or("Received error for unknown subscription")?;

            sink.send(GraphqlClient::error_response(payload)?).await?;
        }
    }

    Ok(())
}

async fn sender_loop<M, S, E, GenericResponse>(
    message_stream: mpsc::Receiver<M>,
    mut ws_sender: S,
    operations: OperationMap<GenericResponse>,
    shutdown: oneshot::Receiver<()>,
) where
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
                    ws_sender.send(msg).await.unwrap();
                } else {
                    return;
                }
            }
            _ = shutdown => {
                // Shutdown the incoming message stream
                let mut message_stream = message_stream.into_inner();
                message_stream.close();
                while message_stream.next().await.is_some() {}

                // Clear out any operations
                operations.lock().await.clear();

                return;
            }
        }
    }
}

struct ClientInner<GraphqlClient>
where
    GraphqlClient: crate::graphql::GraphqlClient,
{
    #[allow(dead_code)]
    receiver_handle: JoinHandle<()>,
    #[allow(dead_code)]
    sender_handle: JoinHandle<()>,
    operations: OperationMap<GraphqlClient::Response>,
}

fn json_message<M: WebsocketMessage>(payload: impl serde::Serialize) -> Result<M, BoxError> {
    Ok(M::new(serde_json::to_string(&payload)?))
}

fn decode_message<T: serde::de::DeserializeOwned, WsMessage: WebsocketMessage>(
    message: WsMessage,
) -> Result<Option<T>, BoxError> {
    if message.is_ping() || message.is_pong() {
        Ok(None)
    } else if message.is_close() {
        todo!()
    } else if let Some(s) = message.text() {
        println!("Received {}", s);
        Ok(Some(serde_json::from_str::<T>(&s)?))
    } else {
        Ok(None)
    }
}
