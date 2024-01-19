use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc,
};

use futures::{
    channel::{mpsc, oneshot},
    SinkExt, StreamExt,
};
use serde_json::Value;

use crate::{
    graphql::GraphqlOperation,
    protocol::{self, Message},
    Error,
};

use self::stream::SubscriptionStream;

mod actor;
mod connection;
mod stream;

pub struct Client {
    actor: mpsc::Sender<ConnectionCommand>,
    subscription_buffer_size: usize,
    next_id: Arc<AtomicUsize>,
}

impl Client {
    // Starts a streaming operation on this client.
    ///
    /// Returns a `Stream` of responses.
    pub async fn streaming_operation<'a, Operation>(
        &mut self,
        op: Operation,
    ) -> Result<SubscriptionStream<Operation>, Error>
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

        self.actor
            .send(ConnectionCommand::Subscribe {
                request,
                sender,
                id,
            })
            .await
            .map_err(|error| Error::Send(error.to_string()))?;

        Ok(SubscriptionStream::<Operation> {
            id,
            stream: Box::pin(receiver.map(move |response| {
                op.decode(response)
                    .map_err(|err| Error::Decode(err.to_string()))
            })),
            actor: self.actor.clone(),
        })
    }

    // TODO: This should be on the stream itself
    // pub fn cancel(&self) -> () {
    //     todo!()
    // }

    pub fn close(&self) -> () {
        todo!()
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
