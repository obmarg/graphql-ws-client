use std::future::Future;
use std::pin::Pin;

use crate::Error;

/// Abstraction around a websocket connection.
///
/// Built in implementations are provided for `ws_stream_wasm` & `async_tungstenite`.
///
/// If users wish to add support for a new client they should implement this trait.
pub trait Connection {
    /// Receive the next message on this connection.
    fn receive(&mut self) -> impl Future<Output = Option<Message>> + Send;

    /// Send a message with on connection
    fn send(&mut self, message: Message) -> impl Future<Output = Result<(), Error>> + Send;
}

/// A websocket message
///
/// Websocket client libraries usually provide their own version of this struct.
/// The [Connection] trait for a given client should handle translation to & from this enum.
pub enum Message {
    /// A message containing the given text payload
    Text(String),
    /// A message that closes the connection with the given code & reason
    Close {
        /// The status code for this close message
        code: Option<u16>,
        /// Some text explaining the reason the connection is being closed
        reason: Option<String>,
    },
    /// A ping
    Ping,
    /// A reply to a ping
    Pong,
}

impl Message {
    pub(crate) fn deserialize<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let Message::Text(text) = self else {
            panic!("Don't call deserialize on non-text messages");
        };

        serde_json::from_str(&text).map_err(|error| Error::Decode(error.to_string()))
    }

    pub(crate) fn init(payload: Option<serde_json::Value>) -> Self {
        Self::Text(
            serde_json::to_string(&crate::protocol::ConnectionInit::new(payload))
                .expect("payload is already serialized so this shouldn't fail"),
        )
    }

    pub(crate) fn graphql_pong() -> Self {
        Self::Text(serde_json::to_string(&crate::protocol::Message::Pong::<()>).unwrap())
    }

    pub(crate) fn graphql_ping() -> Self {
        Self::Text(serde_json::to_string(&crate::protocol::Message::Ping::<()>).unwrap())
    }

    pub(crate) fn complete(id: usize) -> Self {
        Self::Text(
            serde_json::to_string(&crate::protocol::Message::Complete::<()> { id: id.to_string() })
                .unwrap(),
        )
    }
}

/// An object safe wrapper around the Connection trait, allowing us
/// to use it dynamically
pub(crate) trait ObjectSafeConnection: Send {
    fn receive(&mut self) -> Pin<Box<dyn Future<Output = Option<Message>> + Send + '_>>;

    fn send(
        &mut self,
        message: Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + '_>>;
}

impl<T: Connection + Sized + Send> ObjectSafeConnection for T {
    fn receive(&mut self) -> Pin<Box<dyn Future<Output = Option<Message>> + Send + '_>> {
        Box::pin(Connection::receive(self))
    }

    fn send(
        &mut self,
        message: Message,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + '_>> {
        Box::pin(Connection::send(self, message))
    }
}
