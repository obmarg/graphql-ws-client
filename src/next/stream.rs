use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{channel::mpsc, SinkExt, Stream};

use crate::{graphql::GraphqlOperation, Error};

use super::{actor::ConnectionActor, ConnectionCommand};

/// A `futures::Stream` for a subscription.
///
/// Emits an item for each message received by the subscription.
#[pin_project::pin_project]
pub struct SubscriptionStream<Operation>
where
    Operation: GraphqlOperation,
{
    pub(super) id: usize,
    pub(super) stream: Pin<Box<dyn Stream<Item = Result<Operation::Response, Error>> + Send>>,
    pub(super) actor: mpsc::Sender<ConnectionCommand>,
}

impl<Operation> SubscriptionStream<Operation>
where
    Operation: GraphqlOperation + Send,
{
    /// Stops the operation by sending a Complete message to the server.
    pub async fn stop_operation(mut self) -> Result<(), Error> {
        self.actor
            .send(ConnectionCommand::Cancel(self.id))
            .await
            .map_err(|error| todo!())
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
