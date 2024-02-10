//! Contains traits to provide support for various underlying websocket clients.

/// An abstraction around WebsocketMessages
///
/// graphql-ws-client doesn't implement the websocket protocol itself.
/// This trait provides part of the integration with websocket client libraries.
#[deprecated(
    since = "0.8.0-rc.1",
    note = "WebsockeMessage is only used for the deprecated AsyncWebsocketClient.  You should update to use Client instead"
)]
pub trait WebsocketMessage: std::fmt::Debug {
    /// The `Error` type for this websocket client.
    type Error: std::error::Error + Send + 'static;

    /// Constructs a new message with the given text
    fn new(text: String) -> Self;

    /// Returns the text (if there is any) contained in this message
    fn text(&self) -> Option<&str>;

    /// Returns the text (if there is any) of the error
    fn error_message(&self) -> Option<String>;

    /// Returns true if this message is a websocket ping.
    fn is_ping(&self) -> bool;

    /// Returns true if this message is a websocket pong.
    fn is_pong(&self) -> bool;

    /// Returns true if this message is a websocket close.
    fn is_close(&self) -> bool;
}
