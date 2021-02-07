/// An abstraction around WebsocketMessages
///
/// graphql-ws-client doesn't implement the websocket protocol itself.
/// This trait provides part of the integration with websocket client libraries.
pub trait WebsocketMessage: std::fmt::Debug {
    type Error: std::error::Error + Send + 'static;

    fn new(text: String) -> Self;
    fn text(&self) -> Option<&str>;
}
