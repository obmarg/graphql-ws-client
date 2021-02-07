//! Message definitions for the [graphql-transport-ws protocol][1]
//!
//! [1]: https://github.com/enisdenjo/graphql-ws/blob/HEAD/PROTOCOL.md

#[derive(Default, Debug)]
pub struct ConnectionInit<Payload = ()> {
    payload: Option<Payload>,
}

impl<Payload> ConnectionInit<Payload> {
    pub fn new() -> Self {
        ConnectionInit { payload: None }
    }

    pub fn with_payload(payload: Payload) -> Self {
        ConnectionInit {
            payload: Some(payload),
        }
    }
}

impl serde::Serialize for ConnectionInit {
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

pub struct ConnectionAck<Payload = ()> {
    pub payload: Option<Payload>,
}

impl<'de, Payload> serde::Deserialize<'de> for ConnectionAck<Payload>
where
    Payload: serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct ConnectionAckMessage<Payload> {
            r#type: String,
            payload: Option<Payload>,
        }
        let message = ConnectionAckMessage::deserialize(deserializer)?;
        if message.r#type != "connection_ack" {
            return Err(serde::de::Error::custom(format!(
                "expected a connection_ack message, got a {}",
                message.r#type
            )));
        }

        Ok(ConnectionAck {
            payload: message.payload,
        })
    }
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
pub enum Message<'a, Operation> {
    #[serde(rename = "subscribe")]
    Subscribe { id: String, payload: &'a Operation },
    #[serde(rename = "complete")]
    Complete { id: String },
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
}

impl<Response> Event<Response> {
    pub fn id(&self) -> &str {
        match self {
            Event::Next { id, .. } => id.as_ref(),
            Event::Complete { id, .. } => id.as_ref(),
            Event::Error { id, .. } => id.as_ref(),
        }
    }
}

// so, thoughts:
//
// A transport-ws-async feature:
//
// that takes a futures::Sink & futures::Stream
// uses those as the send & sink.
// That'll support async_std _and_ tokio via tungestenite_async
// though it's still the clients job to implement.
//
// A transport-ws-tungstenite impl that uses plain tungstenite?
//
// Though unsure how to manage concurrency with that?  Looks like
// a shit API for concurrency.
