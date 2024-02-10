use std::{
    collections::{hash_map::Entry, HashMap},
    future::IntoFuture,
};

use futures::{channel::mpsc, future::BoxFuture, FutureExt, SinkExt, StreamExt};
use serde_json::{json, Value};

use crate::{logging::trace, protocol::Event, Error};

use super::{
    connection::{Connection, Message},
    ConnectionCommand,
};

#[must_use]
/// The `ConnectionActor` contains the main loop for handling incoming
/// & outgoing messages for a Client.
///
/// This type implements `IntoFuture` and should usually be spawned
/// with an async runtime.
pub struct ConnectionActor {
    client: Option<mpsc::Receiver<ConnectionCommand>>,
    connection: Box<dyn Connection + Send>,
    operations: HashMap<usize, mpsc::Sender<Value>>,
}

impl ConnectionActor {
    pub(super) fn new(
        connection: Box<dyn Connection + Send>,
        client: mpsc::Receiver<ConnectionCommand>,
    ) -> Self {
        ConnectionActor {
            client: Some(client),
            connection,
            operations: HashMap::new(),
        }
    }

    async fn run(mut self) {
        while let Some(next) = self.next().await {
            let response = match next {
                Next::Command(cmd) => self.handle_command(cmd).await,
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

    async fn handle_command(&mut self, cmd: ConnectionCommand) -> Option<Message> {
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
                let id = match event.id().unwrap().parse::<usize>().ok() {
                    Some(id) => id,
                    None => return Some(Message::close(Reason::UnknownSubscription)),
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
                let id = match id.parse::<usize>().ok() {
                    Some(id) => id,
                    None => return Some(Message::close(Reason::UnknownSubscription)),
                };

                trace!("Stream complete");

                self.operations.remove(&id);
                None
            }
            Event::ConnectionAck { .. } => Some(Message::close(Reason::UnexpectedAck)),
            Event::Ping { .. } => Some(Message::Pong),
            Event::Pong { .. } => None,
        }
    }

    async fn next(&mut self) -> Option<Next> {
        loop {
            if let Some(client) = &mut self.client {
                let mut next_command = client.next().fuse();
                let mut next_message = self.connection.receive().fuse();
                futures::select! {
                    command = next_command => {
                        let Some(command) = command else {
                            self.client.take();
                            continue;
                        };

                        return Some(Next::Command(command));
                    },
                    message = next_message => {
                        return Some(Next::Message(message?));
                    },
                }
            }

            if self.operations.is_empty() {
                // If client has disconnected and we have no running operations
                // then we should shut down
                return None;
            }

            return Some(Next::Message(self.connection.receive().await?));
        }
    }
}

enum Next {
    Command(ConnectionCommand),
    Message(Message),
}

impl IntoFuture for ConnectionActor {
    type Output = ();

    type IntoFuture = BoxFuture<'static, ()>;

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
