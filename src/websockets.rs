//! Contains traits to provide support for various underlying websocket clients.

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

    /// Returns the text (if there is any) of the error
    fn error_message(&self) -> Option<String>;

    /// Returns true if this message is a websocket ping.
    fn is_ping(&self) -> bool;

    /// Returns true if this message is a websocket pong.
    fn is_pong(&self) -> bool;

    /// Returns true if this message is a websocket close.
    fn is_close(&self) -> bool;
}

#[cfg(feature = "async-tungstenite")]
impl WebsocketMessage for async_tungstenite::tungstenite::Message {
    type Error = async_tungstenite::tungstenite::Error;

    fn new(text: String) -> Self {
        async_tungstenite::tungstenite::Message::Text(text)
    }

    fn text(&self) -> Option<&str> {
        match self {
            async_tungstenite::tungstenite::Message::Text(text) => Some(text.as_ref()),
            _ => None,
        }
    }

    fn error_message(&self) -> Option<String> {
        match self {
            async_tungstenite::tungstenite::Message::Close(Some(frame)) => {
                Some(frame.reason.to_string())
            }
            _ => None,
        }
    }

    fn is_ping(&self) -> bool {
        matches!(self, async_tungstenite::tungstenite::Message::Ping(_))
    }

    fn is_pong(&self) -> bool {
        matches!(self, async_tungstenite::tungstenite::Message::Pong(_))
    }

    fn is_close(&self) -> bool {
        matches!(self, async_tungstenite::tungstenite::Message::Close(_))
    }
}

#[cfg(feature = "ws_stream_wasm")]
#[derive(Debug)]
/// A WebSocket event abstraction to combine the differentiated WebSocket events and messages of `ws_stream_wasm`.
pub enum WasmWebsocketMessage {
    /// A WebSocket message, text or binary, was received.
    WsMessage(ws_stream_wasm::WsMessage),
    /// A WebSocket event other than a message was received.
    WsEvent(ws_stream_wasm::WsEvent),
}

#[cfg(feature = "ws_stream_wasm")]
impl WebsocketMessage for WasmWebsocketMessage {
    type Error = ws_stream_wasm::WsErr;

    fn new(text: String) -> Self {
        WasmWebsocketMessage::WsMessage(ws_stream_wasm::WsMessage::Text(text))
    }

    fn text(&self) -> Option<&str> {
        match self {
            WasmWebsocketMessage::WsMessage(ws_stream_wasm::WsMessage::Text(text)) => {
                Some(text.as_ref())
            }
            _ => None,
        }
    }

    fn error_message(&self) -> Option<String> {
        match self {
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::WsErr(error)) => {
                Some(error.to_string())
            }
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::Error) => {
                Some("An undisclosed error happened on the connection.".into())
            }
            _ => None,
        }
    }

    fn is_ping(&self) -> bool {
        false
    }

    fn is_pong(&self) -> bool {
        false
    }

    fn is_close(&self) -> bool {
        matches!(
            self,
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::Closed(_))
        )
    }
}
