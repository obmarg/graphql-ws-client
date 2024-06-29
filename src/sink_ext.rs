use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use futures_lite::ready;
use futures_sink::Sink;

/// A very limited clone of [futures::sink::SinkExt](1) to avoid having to
/// pull the original in
///
/// [1]: https://docs.rs/futures/latest/futures/sink/trait.SinkExt.html
pub trait SinkExt<Item>: Sink<Item> {
    fn send(&mut self, item: Item) -> Send<'_, Self, Item>
    where
        Self: Unpin,
    {
        Send::new(self, item)
    }
}

impl<Item, T> SinkExt<Item> for T where T: Sink<Item> {}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Send<'a, Si: ?Sized, Item> {
    sink: &'a mut Si,
    item: Option<Item>,
}

// Pinning is never projected to children
impl<Si: Unpin + ?Sized, Item> Unpin for Send<'_, Si, Item> {}

impl<'a, Si: Sink<Item> + Unpin + ?Sized, Item> Send<'a, Si, Item> {
    pub(super) fn new(sink: &'a mut Si, item: Item) -> Self {
        Self {
            sink,
            item: Some(item),
        }
    }
}

impl<Si: Sink<Item> + Unpin + ?Sized, Item> Future for Send<'_, Si, Item> {
    type Output = Result<(), Si::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let mut sink = Pin::new(&mut this.sink);

        if let Some(item) = this.item.take() {
            ready!(sink.as_mut().poll_ready(cx))?;
            sink.as_mut().start_send(item)?;
        }

        // we're done sending the item, but want to block on flushing the
        // sink
        ready!(sink.poll_flush(cx))?;

        Poll::Ready(Ok(()))
    }
}
