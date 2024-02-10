use futures::{future::BoxFuture, FutureExt, SinkExt, StreamExt};
use pharos::{Observable, ObserveConfig};
use ws_stream_wasm::{WsEvent, WsMessage, WsMeta, WsStream};

use crate::Error;

/// A websocket connection for ws_stream_wasm
#[cfg_attr(docsrs, doc(cfg(feature = "ws_stream_wasm")))]
pub struct Connection {
    messages: WsStream,
    event_stream: pharos::Events<WsEvent>,
    meta: WsMeta,
}

impl Connection {
    /// Creates a new Connection from a WsMeta & WsTream combo
    pub async fn new((mut meta, messages): (WsMeta, WsStream)) -> Self {
        let event_stream = meta.observe(ObserveConfig::default()).await.unwrap();

        Connection {
            messages,
            event_stream,
            meta,
        }
    }
}

impl crate::next::Connection for Connection {
    fn receive(&mut self) -> BoxFuture<'_, Option<crate::next::Message>> {
        Box::pin(async move {
            use crate::next::Message;
            loop {
                match self.next().await? {
                    EventOrMessage::Event(WsEvent::Closed(close)) => {
                        return Some(Message::Close {
                            code: Some(close.code),
                            reason: Some(close.reason),
                        });
                    }
                    EventOrMessage::Event(WsEvent::Error | WsEvent::WsErr(_)) => {
                        return None;
                    }
                    EventOrMessage::Event(WsEvent::Open | WsEvent::Closing) => {
                        continue;
                    }

                    EventOrMessage::Message(WsMessage::Text(text)) => {
                        return Some(Message::Text(text))
                    }

                    EventOrMessage::Message(WsMessage::Binary(_)) => {
                        // We shouldn't receive binary messages, but ignore them if we do
                        continue;
                    }
                }
            }
        })
    }

    fn send(&mut self, message: crate::next::Message) -> BoxFuture<'_, Result<(), Error>> {
        use crate::next::Message;

        Box::pin(async move {
            match message {
                Message::Text(text) => self.messages.send(WsMessage::Text(text)).await,
                Message::Close { code, reason } => match (code, reason) {
                    (Some(code), Some(reason)) => self.meta.close_reason(code, reason).await,
                    (Some(code), _) => self.meta.close_code(code).await,
                    _ => self.meta.close().await,
                }
                .map(|_| ()),
                Message::Ping | Message::Pong => return Ok(()),
            }
            .map_err(|error| Error::Send(error.to_string()))
        })
    }
}

impl Connection {
    async fn next(&mut self) -> Option<EventOrMessage> {
        futures::select! {
            event = self.event_stream.next().fuse() => {
                event.map(EventOrMessage::Event)
            }
            message = self.messages.next().fuse() => {
                message.map(EventOrMessage::Message)
            }
        }
    }
}

enum EventOrMessage {
    Event(WsEvent),
    Message(WsMessage),
}
