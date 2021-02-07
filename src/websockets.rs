//! Contians traits to provide support for various underlying websocket clients.

/// An abstraction around WebsocketMessages
///
/// graphql-ws-client doesn't implement the websocket protocol itself.
/// This trait provides part of the integration with websocket client libraries.
pub trait WebsocketMessage: std::fmt::Debug {
    /// The `Error` type for this websocket client.
    type Error: std::error::Error + Send + 'static;

    /// Constructs a new message with the given text
    fn new(text: String) -> Self;

    /// Returns the text (if there is any) contained in this message
    fn text(&self) -> Option<&str>;
}

#[cfg(feature = "async-tungstenite")]
impl WebsocketMessage for async_tungstenite::tungstenite::Message {
    type Error = async_tungstenite::tungstenite::Error;

    fn new(text: String) -> Self {
        async_tungstenite::tungstenite::Message::Text(text)
    }

    // TODO: This should maybe return Error?
    fn text(&self) -> Option<&str> {
        match self {
            async_tungstenite::tungstenite::Message::Text(text) => Some(text.as_ref()),
            _ => None,
        }
    }
}
