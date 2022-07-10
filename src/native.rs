use crate::websockets::WebsocketMessage;

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
