use std::{
    collections::{hash_map::Entry, HashMap},
    future::IntoFuture,
};

use futures_lite::{future, stream, FutureExt, StreamExt};
use serde_json::{json, Value};

use crate::{
    logging::{trace, warning},
    protocol::Event,
    Error,
};

use super::{
    connection::{Message, ObjectSafeConnection},
    keepalive::KeepAliveSettings,
    ConnectionCommand,
};

#[must_use]
/// The `ConnectionActor` contains the main loop for handling incoming
/// & outgoing messages for a Client.
///
/// This type implements `IntoFuture` and should usually be spawned
/// with an async runtime.
pub struct ConnectionActor {
    client: Option<async_channel::Receiver<ConnectionCommand>>,
    connection: Box<dyn ObjectSafeConnection>,
    operations: HashMap<usize, async_channel::Sender<Value>>,
    keep_alive: KeepAliveSettings,
    keep_alive_actor: stream::Boxed<ConnectionCommand>,
}

impl ConnectionActor {
    pub(super) fn new(
        connection: Box<dyn ObjectSafeConnection>,
        client: async_channel::Receiver<ConnectionCommand>,
        keep_alive: KeepAliveSettings,
    ) -> Self {
        ConnectionActor {
            client: Some(client),
            connection,
            operations: HashMap::new(),
            keep_alive_actor: Box::pin(keep_alive.run()),
            keep_alive,
        }
    }

    async fn run(mut self) {
        while let Some(next) = self.next().await {
            let response = match next {
                Next::Command(cmd) => self.handle_command(cmd),
                Next::Message(message) => self.handle_message(message).await,
            };

            let Some(response) = response else { continue };

            if matches!(response, Message::Close { .. }) {
                self.connection.send(response).await.ok();
                return;
            }

            if self.connection.send(response).await.is_err() {
                return;
            }
        }

        self.connection
            .send(Message::Close {
                code: Some(100),
                reason: None,
            })
            .await
            .ok();
    }

    fn handle_command(&mut self, cmd: ConnectionCommand) -> Option<Message> {
        match cmd {
            ConnectionCommand::Subscribe {
                request,
                sender,
                id,
            } => {
                assert!(self.operations.insert(id, sender).is_none());

                Some(Message::Text(request))
            }
            ConnectionCommand::Cancel(id) => {
                if self.operations.remove(&id).is_some() {
                    return Some(Message::complete(id));
                }
                None
            }
            ConnectionCommand::Close(code, reason) => Some(Message::Close {
                code: Some(code),
                reason: Some(reason),
            }),
            ConnectionCommand::Ping => Some(Message::Ping),
        }
    }

    async fn handle_message(&mut self, message: Message) -> Option<Message> {
        let event = match extract_event(message) {
            Ok(event) => event?,
            Err(Error::Close(code, reason)) => {
                return Some(Message::Close {
                    code: Some(code),
                    reason: Some(reason),
                })
            }
            Err(other) => {
                return Some(Message::Close {
                    code: Some(4857),
                    reason: Some(format!("Error while decoding event: {other}")),
                })
            }
        };

        match event {
            event @ (Event::Next { .. } | Event::Error { .. }) => {
                let Some(id) = event.id().unwrap().parse::<usize>().ok() else {
                    return Some(Message::close(Reason::UnknownSubscription));
                };

                let sender = self.operations.entry(id);

                let Entry::Occupied(mut sender) = sender else {
                    return None;
                };

                let payload = event.forwarding_payload().unwrap();

                if sender.get_mut().send(payload).await.is_err() {
                    sender.remove();
                    return Some(Message::complete(id));
                }

                None
            }
            Event::Complete { id } => {
                let Some(id) = id.parse::<usize>().ok() else {
                    return Some(Message::close(Reason::UnknownSubscription));
                };

                trace!("Stream complete");

                self.operations.remove(&id);
                None
            }
            Event::ConnectionAck { .. } => Some(Message::close(Reason::UnexpectedAck)),
            Event::Ping { .. } => Some(Message::graphql_pong()),
            Event::Pong { .. } => None,
        }
    }

    async fn next(&mut self) -> Option<Next> {
        enum Select {
            Command(Option<ConnectionCommand>),
            Message(Option<Message>),
            KeepAlive(Option<ConnectionCommand>),
        }
        loop {
            if let Some(client) = &mut self.client {
                let command = async { Select::Command(client.recv().await.ok()) };
                let message = async { Select::Message(self.connection.receive().await) };
                let keep_alive = async { Select::KeepAlive(self.keep_alive_actor.next().await) };

                match command.or(message).or(keep_alive).await {
                    Select::Command(Some(command)) | Select::KeepAlive(Some(command)) => {
                        return Some(Next::Command(command));
                    }
                    Select::Command(None) => {
                        self.client.take();
                        continue;
                    }
                    Select::Message(message) => {
                        self.keep_alive_actor = Box::pin(self.keep_alive.run());
                        return Some(Next::Message(message?));
                    }
                    Select::KeepAlive(None) => {
                        return Some(self.keep_alive.report_timeout());
                    }
                }
            }

            if self.operations.is_empty() {
                // If client has disconnected and we have no running operations
                // then we should shut down
                return None;
            }
        }
    }
}

enum Next {
    Command(ConnectionCommand),
    Message(Message),
}

impl IntoFuture for ConnectionActor {
    type Output = ();

    type IntoFuture = future::Boxed<()>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.run())
    }
}

fn extract_event(message: Message) -> Result<Option<Event>, Error> {
    match message {
        Message::Text(s) => {
            trace!("Decoding message: {}", s);
            Ok(Some(
                serde_json::from_str(&s).map_err(|err| Error::Decode(err.to_string()))?,
            ))
        }
        Message::Close { code, reason } => Err(Error::Close(
            code.unwrap_or_default(),
            reason.unwrap_or_default(),
        )),
        Message::Ping | Message::Pong => Ok(None),
    }
}

enum Reason {
    UnexpectedAck,
    UnknownSubscription,
}

impl Message {
    fn close(reason: Reason) -> Self {
        match reason {
            Reason::UnexpectedAck => Message::Close {
                code: Some(4855),
                reason: Some("too many acknowledges".into()),
            },
            Reason::UnknownSubscription => Message::Close {
                code: Some(4856),
                reason: Some("unknown subscription".into()),
            },
        }
    }
}

impl Event {
    fn forwarding_payload(self) -> Option<Value> {
        match self {
            Event::Next { payload, .. } => Some(payload),
            Event::Error { payload, .. } => Some(json!({"errors": payload})),
            _ => None,
        }
    }
}

impl KeepAliveSettings {
    fn report_timeout(&self) -> Next {
        warning!(
            "No messages received within keep-alive ({:?}s) from server. Closing the connection",
            self.interval.unwrap()
        );
        Next::Command(ConnectionCommand::Close(
            4503,
            "Service unavailable. keep-alive failure".to_string(),
        ))
    }
}
