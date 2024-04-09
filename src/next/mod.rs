use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures::{channel::mpsc, SinkExt, StreamExt};
use serde_json::Value;

use crate::{
    graphql::GraphqlOperation,
    protocol::{self},
    Error,
};

mod actor;
mod builder;
mod connection;
mod stream;

pub use self::{
    actor::ConnectionActor,
    builder::ClientBuilder,
    connection::{Connection, Message},
    stream::Subscription,
};

/// A GraphQL over Websocket client
///
/// ```rust,no_run
/// use graphql_ws_client::Client;
/// use std::future::IntoFuture;
/// use futures::StreamExt;
/// # use graphql_ws_client::__doc_utils::spawn;
/// # async fn example() -> Result<(), graphql_ws_client::Error> {
/// # let connection = graphql_ws_client::__doc_utils::Conn;
/// # let subscription = graphql_ws_client::__doc_utils::Subscription;
///
/// let (mut client, actor) = Client::build(connection).await?;
///
/// // Spawn the actor onto an async runtime
/// spawn(actor.into_future());
///
/// let mut subscription = client.subscribe(subscription).await?;
///
/// while let Some(response) = subscription.next().await {
///     // Do something with response
/// }
/// # Ok(())
/// # }
#[derive(Clone)]
pub struct Client {
    actor: mpsc::Sender<ConnectionCommand>,
    subscription_buffer_size: usize,
    next_id: Arc<AtomicUsize>,
}

impl Client {
    pub(super) fn new_internal(
        actor: mpsc::Sender<ConnectionCommand>,
        subscription_buffer_size: usize,
    ) -> Self {
        Client {
            actor,
            subscription_buffer_size,
            next_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    // Starts a streaming operation on this client.
    ///
    /// Returns a `Stream` of responses.
    pub async fn subscribe<'a, Operation>(
        &self,
        op: Operation,
    ) -> Result<Subscription<Operation>, Error>
    where
        Operation: GraphqlOperation + Unpin + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(self.subscription_buffer_size);

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let message = protocol::Message::Subscribe {
            id: id.to_string(),
            payload: &op,
        };

        let request = serde_json::to_string(&message)
            .map_err(|error| Error::Serializing(error.to_string()))?;

        let mut actor = self.actor.clone();
        actor
            .send(ConnectionCommand::Subscribe {
                request,
                sender,
                id,
            })
            .await
            .map_err(|error| Error::Send(error.to_string()))?;

        Ok(Subscription::<Operation> {
            id,
            stream: Box::pin(receiver.map(move |response| {
                op.decode(response)
                    .map_err(|err| Error::Decode(err.to_string()))
            })),
            actor,
        })
    }

    /// Gracefully closes the connection
    ///
    /// This will stop all running subscriptions and shut down the ConnectionActor wherever
    /// it is running.
    pub async fn close(mut self, code: u16, description: impl Into<String>) {
        self.actor
            .send(ConnectionCommand::Close(code, description.into()))
            .await
            .ok();
    }
}

pub(super) enum ConnectionCommand {
    Subscribe {
        /// The full subscribe request as a JSON encoded string.
        request: String,
        sender: mpsc::Sender<Value>,
        id: usize,
    },
    Cancel(usize),
    Close(u16, String),
}
