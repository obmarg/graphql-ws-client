use futures_lite::{Stream, StreamExt};
use futures_sink::Sink;
use tungstenite_0_27::{self as tungstenite, Bytes, protocol::CloseFrame};

use crate::{Error, Message, sink_ext::SinkExt};

#[cfg_attr(docsrs, doc(cfg(feature = "tungstenite-0.28")))]
impl<T> crate::client::Connection for T
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
                    return Some(crate::client::Message::Text(text.as_str().into()));
                }
                Ok(tungstenite::Message::Ping(_)) => return Some(crate::client::Message::Ping),
                Ok(tungstenite::Message::Pong(_)) => return Some(crate::client::Message::Pong),
                Ok(tungstenite::Message::Close(frame)) => {
                    return Some(crate::client::Message::Close {
                        code: frame.as_ref().map(|frame| frame.code.into()),
                        reason: frame.map(|frame| frame.reason.to_string()),
                    });
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

    async fn send(&mut self, message: crate::client::Message) -> Result<(), Error> {
        <Self as SinkExt<tungstenite::Message>>::send(
            self,
            match message {
                crate::client::Message::Text(text) => tungstenite::Message::Text(text.into()),
                crate::client::Message::Close { code, reason } => {
                    tungstenite::Message::Close(code.zip(reason).map(|(code, reason)| CloseFrame {
                        code: code.into(),
                        reason: reason.into(),
                    }))
                }
                crate::client::Message::Ping => tungstenite::Message::Ping(Bytes::new()),
                crate::client::Message::Pong => tungstenite::Message::Pong(Bytes::new()),
            },
        )
        .await
        .map_err(|error| Error::Send(error.to_string()))
    }
}
