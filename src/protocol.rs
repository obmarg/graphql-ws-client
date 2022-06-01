//! Message definitions for the [graphql-transport-ws protocol][1]
//!
//! [1]: https://github.com/enisdenjo/graphql-ws/blob/HEAD/PROTOCOL.md

#[derive(Default, Debug)]
pub struct ConnectionInit<Payload = ()> {
    payload: Option<Payload>,
}

impl<Payload> ConnectionInit<Payload> {
    pub fn new(payload: Option<Payload>) -> Self {
        ConnectionInit { payload }
    }
}

impl<Payload> serde::Serialize for ConnectionInit<Payload>
where
    Payload: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", "connection_init")?;
        if self.payload.is_some() {
            map.serialize_entry("payload", &self.payload)?;
        }
        map.end()
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
pub enum Message<'a, Operation> {
    #[serde(rename = "subscribe")]
    Subscribe { id: String, payload: &'a Operation },
    #[serde(rename = "complete")]
    #[allow(dead_code)]
    Complete { id: String },
    #[serde(rename = "pong")]
    Pong,
}

#[derive(serde::Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Event<Response> {
    #[serde(rename = "next")]
    Next { id: String, payload: Response },
    #[serde(rename = "error")]
    Error {
        id: String,
        payload: Vec<serde_json::Value>,
    },
    #[serde(rename = "complete")]
    Complete { id: String },
    #[serde(rename = "connection_ack")]
    ConnectionAck { payload: Option<serde_json::Value> },
    #[serde(rename = "ping")]
    Ping { payload: Option<serde_json::Value> },
    #[serde(rename = "pong")]
    Pong { payload: Option<serde_json::Value> },
}

impl<Response> Event<Response> {
    pub fn id(&self) -> Option<&str> {
        match self {
            Event::Next { id, .. } => Some(id.as_ref()),
            Event::Complete { id, .. } => Some(id.as_ref()),
            Event::Error { id, .. } => Some(id.as_ref()),
            Event::Ping { .. } | Event::Pong { .. } | Event::ConnectionAck { .. } => None,
        }
    }

    pub fn r#type(&self) -> &'static str {
        match self {
            Event::Next { .. } => "next",
            Event::Complete { .. } => "complete",
            Event::Error { .. } => "error",
            Event::Ping { .. } => "ping",
            Event::Pong { .. } => "pong",
            Event::ConnectionAck { .. } => "connection_ack",
        }
    }
}
