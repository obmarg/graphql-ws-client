use std::future::Future;

use serde::Serialize;
use serde_json::json;

#[async_trait::async_trait]
pub trait Connection {
    async fn receive(&self) -> Result<Message, ()>;
    async fn send(&self, message: Message) -> Result<(), ()>;
}

pub enum Message {
    Text(String),
    Close {
        code: Option<u16>,
        reason: Option<String>,
    },
    Ping,
    Pong,
}

impl Message {
    pub(crate) fn complete(id: usize) -> Self {
        Self::Text(
            serde_json::to_string(&crate::protocol::Message::Complete::<()> { id: id.to_string() })
                .unwrap(),
        )
    }
}
