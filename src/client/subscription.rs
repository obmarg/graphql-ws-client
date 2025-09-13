use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_lite::{Stream, StreamExt, future, stream};

use crate::{
    Error, SubscriptionId, client::production_future::read_from_producer, graphql::GraphqlOperation,
};

use super::ConnectionCommand;

/// A `futures::Stream` for a subscription.
///
/// Emits an item for each message received by the subscription.
#[pin_project::pin_project(PinnedDrop)]
pub struct Subscription<Operation>
where
    Operation: GraphqlOperation,
{
    pub(in crate::client) id: SubscriptionId,
    pub(in crate::client) stream: Option<stream::Boxed<Result<Operation::Response, Error>>>,
    pub(in crate::client) actor: async_channel::Sender<ConnectionCommand>,
    pub(in crate::client) drop_sender: Option<async_channel::Sender<SubscriptionId>>,
}

#[pin_project::pinned_drop]
impl<Operation> PinnedDrop for Subscription<Operation>
where
    Operation: GraphqlOperation,
{
    fn drop(mut self: Pin<&mut Self>) {
        let Some(drop_sender) = self.drop_sender.take() else {
            return;
        };
        // We try_send here but the drop_sender channel _should_ be unbounded so
        // this should always work if the connection actor is still alive.
        drop_sender.try_send(self.id).ok();
    }
}

impl<Operation> Subscription<Operation>
where
    Operation: GraphqlOperation + Send,
{
    /// Returns the identifier for this subscription.
    ///
    /// This can be used with [`crate::Client::stop`] to stop
    /// a running subscription without needing access to the `Subscription`
    /// itself.
    pub fn id(&self) -> SubscriptionId {
        self.id
    }

    /// Stops this subscription
    pub fn stop(mut self) {
        let Some(drop_sender) = self.drop_sender.take() else {
            return;
        };

        // We try_send here but the drop_sender channel _should_ be unbounded so
        // this should always work if the connection actor is still alive.
        drop_sender.try_send(self.id).ok();
    }

    pub(super) fn join(mut self, future: future::Boxed<()>) -> Self
    where
        Operation::Response: 'static,
    {
        self.stream = self
            .stream
            .take()
            .map(|stream| join_stream(stream, future).boxed());
        self
    }
}

impl<Operation> Stream for Subscription<Operation>
where
    Operation: GraphqlOperation + Unpin,
{
    type Item = Result<Operation::Response, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().stream.as_mut() {
            None => Poll::Ready(None),
            Some(stream) => stream.poll_next(cx),
        }
    }
}

/// Joins a future onto the execution of a stream returning a stream that also polls
/// the given future.
///
/// If the future ends the stream will still continue till completion but if the stream
/// ends the future will be cancelled.
///
/// This can be used when you have the receivng side of a channel and a future that sends
/// on that channel - combining the two into a single stream that'll run till the channel
/// is exhausted.  If you drop the stream you also cancel the underlying process.
fn join_stream<Item>(
    stream: stream::Boxed<Item>,
    future: future::Boxed<()>,
) -> impl Stream<Item = Item> {
    stream::unfold(ProducerState::Running(stream, future), producer_handler)
}

enum ProducerState<'a, Item> {
    Running(
        Pin<Box<dyn Stream<Item = Item> + Send + 'a>>,
        future::Boxed<()>,
    ),
    Draining(Pin<Box<dyn Stream<Item = Item> + Send + 'a>>),
}

async fn producer_handler<Item>(
    mut state: ProducerState<'_, Item>,
) -> Option<(Item, ProducerState<'_, Item>)> {
    loop {
        match state {
            ProducerState::Running(mut stream, producer) => {
                match read_from_producer(stream.next(), producer).await {
                    Some((item, producer)) => {
                        return Some((item?, ProducerState::Running(stream, producer)));
                    }
                    None => state = ProducerState::Draining(stream),
                }
            }
            ProducerState::Draining(mut stream) => {
                return Some((stream.next().await?, ProducerState::Draining(stream)));
            }
        }
    }
}
