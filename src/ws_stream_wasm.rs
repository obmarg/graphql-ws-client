use futures_lite::{FutureExt, StreamExt};
use pharos::{Observable, ObserveConfig};
use ws_stream_wasm::{WsEvent, WsMessage, WsMeta, WsStream};

use crate::{sink_ext::SinkExt, Error};

/// A websocket connection for [ws_stream_wasm][1]
///
/// [1]: https://docs.rs/ws_stream/latest/ws_stream
#[cfg_attr(docsrs, doc(cfg(feature = "ws_stream_wasm")))]
pub struct Connection {
    messages: WsStream,
    event_stream: pharos::Events<WsEvent>,
    meta: WsMeta,
}

impl Connection {
    /// Creates a new Connection from a [`WsMeta`] and [`WsStream`] combo
    ///
    /// # Panics
    ///
    /// Will panic if `meta.observe` fails.
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
    async fn receive(&mut self) -> Option<crate::next::Message> {
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
                EventOrMessage::Message(WsMessage::Text(text)) => return Some(Message::Text(text)),
                EventOrMessage::Message(WsMessage::Binary(_)) => {
                    // We shouldn't receive binary messages, but ignore them if we do
                    continue;
                }
            }
        }
    }

    async fn send(&mut self, message: crate::next::Message) -> Result<(), Error> {
        use crate::next::Message;

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
    }
}

impl Connection {
    async fn next(&mut self) -> Option<EventOrMessage> {
        let event = async { self.event_stream.next().await.map(EventOrMessage::Event) };
        let message = async { self.messages.next().await.map(EventOrMessage::Message) };

        event.race(message).await
    }
}

enum EventOrMessage {
    Event(WsEvent),
    Message(WsMessage),
}
