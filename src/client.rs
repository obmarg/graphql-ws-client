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

// TODO: Make this customisable somehow.
const SUBSCRIPTION_BUFFER_SIZE: usize = 5;

pub struct AsyncWebsocketClient<Message, GraphqlClient>
where
    GraphqlClient: graphql::GraphqlClient,
{
    inner: Arc<ClientInner<GraphqlClient>>,
    sender_sink: mpsc::Sender<Message>,
    phantom: PhantomData<*const GraphqlClient>,
}

// TODO: Better error (somehow)
#[derive(thiserror::Error, Debug)]
#[error("Something went wrong")]
pub struct Error {}

// TODO: Docstrings, book etc.

impl<WsMessage, GraphqlClient> AsyncWebsocketClient<WsMessage, GraphqlClient>
where
    WsMessage: WebsocketMessage + Send + 'static,
    GraphqlClient: crate::graphql::GraphqlClient + 'static,
{
    pub async fn new(
        mut websocket_stream: impl Stream<Item = Result<WsMessage, WsMessage::Error>>
            + Unpin
            + Send
            + 'static,
        mut websocket_sink: impl Sink<WsMessage, Error = WsMessage::Error> + Unpin + Send + 'static,
        runtime: impl SpawnHandle<()>,
    ) -> Result<Self, Error> {
        // TODO: actual error handling, ditch unwraps

        websocket_sink
            .send(json_message(ConnectionInit::new()).unwrap())
            .await
            .map_err(|_| Error {})?;

        match websocket_stream.next().await {
            None => todo!(),
            Some(Err(_)) => todo!(),
            Some(Ok(data)) => {
                decode_message::<ConnectionAck<()>, _>(data).unwrap();
                println!("Connection acked");
            }
        }

        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

        let operations = Arc::new(Mutex::new(HashMap::new()));

        // TODO: Don't force the graphql error type here to be ()
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

    pub async fn streaming_operation<Operation>(
        &mut self,
        op: Operation,
    ) -> impl Stream<Item = Operation::Response>
    where
        Operation: GraphqlOperation<GenericResponse = GraphqlClient::Response>,
    {
        // TODO: no unwraps
        let id = Uuid::new_v4();
        let (sender, receiver) = mpsc::channel(SUBSCRIPTION_BUFFER_SIZE);

        // TODO: THink I need to move operations map into the futures
        self.inner.operations.lock().await.insert(id, sender);

        let msg = json_message(Message::Subscribe {
            id: id.to_string(),
            payload: &op,
        })
        .unwrap();

        self.sender_sink.send(msg).await.unwrap();

        // TODO: This needs to return a type that
        // has close & some sort of status func on it.
        // Have the receiver send details and have that intercepted
        // by this type and stored.
        receiver.map(move |response| op.decode(response).unwrap())
    }
}

type OperationSender<GenericResponse> = mpsc::Sender<GenericResponse>;

type OperationMap<GenericResponse> = Arc<Mutex<HashMap<Uuid, OperationSender<GenericResponse>>>>;

// TODO: Think about whether there's actually some Arc cycles here
// that I need to care about

async fn receiver_loop<S, WsMessage, GraphqlClient>(
    mut receiver: S,
    operations: OperationMap<GraphqlClient::Response>,
    shutdown: oneshot::Sender<()>,
) where
    S: Stream<Item = Result<WsMessage, WsMessage::Error>> + Unpin,
    WsMessage: WebsocketMessage,
    GraphqlClient: crate::graphql::GraphqlClient,
{
    // TODO: Ok, so basically need a oneshot from here -> sender that
    // tells the sender to stop.  It can close it's incoming, drain it's stream
    // and then close the streams in the HashMap.
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

            // TODO: Guess I need a way of constructing a GQL response here
            // to send it via the sink?
            // Could just define our own GraphQLResponse type?
            sink.send(GraphqlClient::error_response(payload)?).await?;
        }
    }

    Ok(())
}

async fn sender_loop<M, S, E, GenericResponse>(
    // TODO: Maybe don't use Message or M here - have this func transform
    // so the type param doesn't escape this func
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
            // TODO: Could use select_next_some here?
            msg = message_stream.next() => {
                if let Some(msg) = msg {
                    println!("Sending message: {:?}", msg);
                    ws_sender.send(msg).await.unwrap();
                } else {
                    // TODO: Do I need to indicate errors in here to the rest of the system?
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
) -> Result<T, BoxError> {
    if let Some(s) = message.text() {
        println!("Received {}", s);
        Ok(serde_json::from_str::<T>(&s)?)
    } else {
        todo!()
    }
}

impl WebsocketMessage for async_tungstenite::tungstenite::Message {
    type Error = async_tungstenite::tungstenite::Error;

    fn new(text: String) -> Self {
        async_tungstenite::tungstenite::Message::Text(text)
    }

    // TODO: This should maybe return Error?
    fn text(&self) -> Option<&str> {
        match self {
            async_tungstenite::tungstenite::Message::Text(text) => Some(text.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: Need to allow the client to just use stream & sink directly.
    // That way I can impl tests for it indepdendant of tungsten stuff...

    // TODO: tests of shutdown behaviour etc would be good.
    // also mocked tests and what not
}
