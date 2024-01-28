use std::future::Future;

use serde::Serialize;
use serde_json::json;

use crate::{protocol, Error};

#[async_trait::async_trait]
pub trait Connection {
    async fn receive(&mut self) -> Option<Message>;
    async fn send(&mut self, message: Message) -> Result<(), Error>;
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

    pub(crate) fn complete(id: usize) -> Self {
        Self::Text(
            serde_json::to_string(&crate::protocol::Message::Complete::<()> { id: id.to_string() })
                .unwrap(),
        )
    }
}
