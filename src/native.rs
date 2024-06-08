use futures_lite::{Stream, StreamExt};
use futures_sink::Sink;
use tungstenite::{self, protocol::CloseFrame};

use crate::{sink_ext::SinkExt, Error, Message};

#[cfg_attr(docsrs, doc(cfg(feature = "tungstenite")))]
impl<T> crate::next::Connection for T
where
    T: Stream<Item = Result<tungstenite::Message, tungstenite::Error>>
        + Sink<tungstenite::Message>
        + Send
        + Sync
        + Unpin,
    <T as Sink<tungstenite::Message>>::Error: std::fmt::Display,
{
    async fn receive(&mut self) -> Option<Message> {
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
                Ok(tungstenite::Message::Frame(_) | tungstenite::Message::Binary(_)) => continue,
                Err(error) => {
                    #[allow(unused)]
                    let error = error;
                    crate::logging::warning!("error receiving message: {error:?}");
                    return None;
                }
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
