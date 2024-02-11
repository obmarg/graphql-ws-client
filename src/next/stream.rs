use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{
    channel::mpsc,
    future::{self, BoxFuture, Fuse},
    stream::{self, BoxStream},
    SinkExt, Stream, StreamExt,
};

use crate::{graphql::GraphqlOperation, Error};

use super::ConnectionCommand;

/// A `futures::Stream` for a subscription.
///
/// Emits an item for each message received by the subscription.
#[pin_project::pin_project]
pub struct Subscription<Operation>
where
    Operation: GraphqlOperation,
{
    pub(super) id: usize,
    pub(super) stream: BoxStream<'static, Result<Operation::Response, Error>>,
    pub(super) actor: mpsc::Sender<ConnectionCommand>,
}

impl<Operation> Subscription<Operation>
where
    Operation: GraphqlOperation + Send,
{
    /// Stops the subscription by sending a Complete message to the server.
    pub async fn stop(mut self) -> Result<(), Error> {
        self.actor
            .send(ConnectionCommand::Cancel(self.id))
            .await
            .map_err(|error| Error::Send(error.to_string()))
    }

    pub(super) fn join(self, future: Fuse<BoxFuture<'static, ()>>) -> Self
    where
        Operation::Response: 'static,
    {
        Self {
            stream: join_stream(self.stream, future).boxed(),
            ..self
        }
    }
}

impl<Operation> Stream for Subscription<Operation>
where
    Operation: GraphqlOperation + Unpin,
{
    type Item = Result<Operation::Response, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.as_mut().poll_next(cx)
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
    stream: BoxStream<'static, Item>,
    future: Fuse<BoxFuture<'static, ()>>,
) -> impl Stream<Item = Item> {
    futures::stream::unfold(
        ProducerState::Running(stream.fuse(), future),
        producer_handler,
    )
}

enum ProducerState<'a, Item> {
    Running(
        stream::Fuse<BoxStream<'a, Item>>,
        future::Fuse<BoxFuture<'a, ()>>,
    ),
    Draining(stream::Fuse<BoxStream<'a, Item>>),
}

async fn producer_handler<Item>(
    mut state: ProducerState<'_, Item>,
) -> Option<(Item, ProducerState<'_, Item>)> {
    loop {
        match state {
            ProducerState::Running(mut stream, mut producer) => {
                futures::select! {
                    output = stream.next() => {
                        return Some((output?, ProducerState::Running(stream, producer)));
                    }
                    _ = producer => {
                        state = ProducerState::Draining(stream);
                        continue;
                    }
                }
            }
            ProducerState::Draining(mut stream) => {
                return Some((stream.next().await?, ProducerState::Draining(stream)));
            }
        }
    }
}
