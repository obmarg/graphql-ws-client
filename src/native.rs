use async_tungstenite::tungstenite::{self, protocol::CloseFrame};
use futures::{AsyncRead, AsyncWrite, SinkExt, StreamExt};

use crate::{websockets::WebsocketMessage, Error};

impl WebsocketMessage for tungstenite::Message {
    type Error = tungstenite::Error;

    fn new(text: String) -> Self {
        tungstenite::Message::Text(text)
    }

    fn text(&self) -> Option<&str> {
        match self {
            tungstenite::Message::Text(text) => Some(text.as_ref()),
            _ => None,
        }
    }

    fn error_message(&self) -> Option<String> {
        match self {
            tungstenite::Message::Close(Some(frame)) => Some(frame.reason.to_string()),
            _ => None,
        }
    }

    fn is_ping(&self) -> bool {
        matches!(self, tungstenite::Message::Ping(_))
    }

    fn is_pong(&self) -> bool {
        matches!(self, tungstenite::Message::Pong(_))
    }

    fn is_close(&self) -> bool {
        matches!(self, tungstenite::Message::Close(_))
    }
}

#[async_trait::async_trait]
impl<T> crate::next::Connection for async_tungstenite::WebSocketStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin + Send + Sync,
{
    async fn receive(&mut self) -> Option<crate::next::Message> {
        loop {
            let message = self.next().await?.expect("TODO: error handling");
            match message {
                tungstenite::Message::Text(text) => return Some(crate::next::Message::Text(text)),
                tungstenite::Message::Ping(_) => return Some(crate::next::Message::Ping),
                tungstenite::Message::Pong(_) => return Some(crate::next::Message::Pong),
                tungstenite::Message::Close(frame) => {
                    return Some(crate::next::Message::Close {
                        code: frame.as_ref().map(|frame| frame.code.into()),
                        reason: frame.map(|frame| frame.reason.to_string()),
                    })
                }
                tungstenite::Message::Frame(_) | tungstenite::Message::Binary(_) => continue,
            }
        }
    }

    async fn send(&mut self, message: crate::next::Message) -> Result<(), Error> {
        <Self as SinkExt<tungstenite::Message>>::send(
            self,
            match message {
                crate::next::Message::Text(text) => tungstenite::Message::Text(text),
                crate::next::Message::Close { code, reason } => {
                    tungstenite::Message::Close(code.zip(reason).map(|(code, reason)| CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    }))
                }
                crate::next::Message::Ping => tungstenite::Message::Ping(vec![]),
                crate::next::Message::Pong => tungstenite::Message::Pong(vec![]),
            },
        )
        .await
        .map_err(|error| Error::Send(error.to_string()))
    }
}
