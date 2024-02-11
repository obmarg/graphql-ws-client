use futures::{future::BoxFuture, Sink, SinkExt, Stream, StreamExt};
use tungstenite::{self, protocol::CloseFrame};

use crate::Error;

#[allow(deprecated)]
impl crate::legacy::websockets::WebsocketMessage for tungstenite::Message {
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

impl<T> crate::next::Connection for T
where
    T: Stream<Item = Result<tungstenite::Message, tungstenite::Error>>
        + Sink<tungstenite::Message>
        + Send
        + Sync
        + Unpin,
    <T as Sink<tungstenite::Message>>::Error: std::fmt::Display,
{
    fn receive(&mut self) -> BoxFuture<'_, Option<crate::next::Message>> {
        Box::pin(async move {
            loop {
                match self.next().await? {
                    Ok(tungstenite::Message::Text(text)) => {
                        return Some(crate::next::Message::Text(text))
                    }
                    Ok(tungstenite::Message::Ping(_)) => return Some(crate::next::Message::Ping),
                    Ok(tungstenite::Message::Pong(_)) => return Some(crate::next::Message::Pong),
                    Ok(tungstenite::Message::Close(frame)) => {
                        return Some(crate::next::Message::Close {
                            code: frame.as_ref().map(|frame| frame.code.into()),
                            reason: frame.map(|frame| frame.reason.to_string()),
                        })
                    }
                    Ok(tungstenite::Message::Frame(_) | tungstenite::Message::Binary(_)) => {
                        continue
                    }
                    Err(error) => {
                        #[allow(unused)]
                        let error = error;
                        crate::logging::warning!("error receiving message: {error:?}");
                        return None;
                    }
                }
            }
        })
    }

    fn send(&mut self, message: crate::next::Message) -> BoxFuture<'_, Result<(), Error>> {
        Box::pin(async move {
            <Self as SinkExt<tungstenite::Message>>::send(
                self,
                match message {
                    crate::next::Message::Text(text) => tungstenite::Message::Text(text),
                    crate::next::Message::Close { code, reason } => {
                        tungstenite::Message::Close(code.zip(reason).map(|(code, reason)| {
                            CloseFrame {
                                code: code.into(),
                                reason: reason.into(),
                            }
                        }))
                    }
                    crate::next::Message::Ping => tungstenite::Message::Ping(vec![]),
                    crate::next::Message::Pong => tungstenite::Message::Pong(vec![]),
                },
            )
            .await
            .map_err(|error| Error::Send(error.to_string()))
        })
    }
}
