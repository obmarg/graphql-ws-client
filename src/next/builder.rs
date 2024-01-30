use std::collections::HashMap;

use futures::channel::mpsc;
use serde::Serialize;

use crate::{logging::trace, protocol::Event, Error};

use super::{
    actor::ConnectionActor,
    connection::{Connection, Message},
    Client,
};

/// A websocket client builder
#[derive(Default)]
pub struct ClientBuilder {
    payload: Option<serde_json::Value>,
    subscription_buffer_size: Option<usize>,
}

impl ClientBuilder {
    /// Constructs an AsyncWebsocketClientBuilder
    pub fn new() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// Add payload to `connection_init`
    pub fn payload<NewPayload>(self, payload: NewPayload) -> Result<ClientBuilder, Error>
    where
        NewPayload: Serialize,
    {
        Ok(ClientBuilder {
            payload: Some(
                serde_json::to_value(payload)
                    .map_err(|error| Error::Serializing(error.to_string()))?,
            ),
            ..self
        })
    }

    pub fn subscription_buffer_size(self, new: usize) -> Self {
        ClientBuilder {
            subscription_buffer_size: Some(new),
            ..self
        }
    }
}

impl ClientBuilder {
    /// Constructs a Client
    ///
    /// Accepts an already built websocket connection, and returns the connection
    /// and a future that must be awaited somewhere - if the future is dropped the
    /// connection will also drop.
    pub async fn build<Conn>(self, connection: Conn) -> Result<(Client, ConnectionActor), Error>
    where
        Conn: Connection + Send + 'static,
    {
        self.build_impl(Box::new(connection)).await
    }

    async fn build_impl(
        self,
        mut connection: Box<dyn Connection + Send>,
    ) -> Result<(Client, ConnectionActor), Error> {
        connection.send(Message::init(self.payload)).await?;

        // wait for ack before entering receiver loop:
        loop {
            match connection.receive().await {
                None => return Err(Error::Unknown("connection dropped".into())),
                Some(Message::Close { code, reason }) => {
                    return Err(Error::Close(
                        code.unwrap_or_default(),
                        reason.unwrap_or_default(),
                    ))
                }
                Some(Message::Ping) | Some(Message::Pong) => {}
                Some(message @ Message::Text(_)) => {
                    let event = message.deserialize::<Event>()?;
                    match event {
                        // pings can be sent at any time
                        Event::Ping { .. } => {
                            connection.send(Message::graphql_pong()).await?;
                        }
                        Event::Pong { .. } => {}
                        Event::ConnectionAck { .. } => {
                            // handshake completed, ready to enter main receiver loop
                            trace!("connection_ack received, handshake completed");
                            break;
                        }
                        event => {
                            connection.send(Message::Close {
                                code: Some(4950),
                                reason: Some("Unexpected message while waiting for ack".into()),
                            });
                            return Err(Error::Decode(format!(
                                "expected a connection_ack or ping, got {}",
                                event.r#type()
                            )));
                        }
                    }
                }
            }
        }

        let (command_sender, command_receiver) = mpsc::channel(5);

        let actor = ConnectionActor::new(connection, command_receiver);

        let client = Client::new(command_sender, self.subscription_buffer_size.unwrap_or(5));

        Ok((client, actor))
    }
}
