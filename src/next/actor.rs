use std::{
    collections::HashMap,
    future::{Future, IntoFuture},
};

use futures::{channel::mpsc, future::BoxFuture, FutureExt, SinkExt, StreamExt};
use serde_json::{json, Value};

use crate::{logging::trace, protocol::Event, Error};

use super::{
    connection::{Connection, Message},
    ConnectionCommand,
};

#[must_use]
pub(crate) struct ConnectionActor {
    connection: Box<dyn Connection + Send>,
    commands: mpsc::Receiver<ConnectionCommand>,
    operations: HashMap<usize, mpsc::Sender<Value>>,
}

impl ConnectionActor {
    // TODO: this could be public as an alternative to IntoFuture
    async fn run(mut self) {
        // TODO: Do the initial ack flow?

        while let Some(next) = self.next().await {
            let response = match next {
                Next::Command(cmd) => self.handle_command(cmd).await,
                Next::Message(message) => self.handle_message(message).await,
            };

            if let Some(response) = response {
                match response {
                    response @ Message::Close { .. } => {
                        self.connection.send(response).await.ok();
                        return;
                    }
                    response => {
                        if self.connection.send(response).await.is_err() {
                            todo!("error handling")
                        }
                    }
                }
            }
        }
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
            Err(error) => todo!(),
        };

        match event {
            event @ (Event::Next { .. } | Event::Error { .. }) => {
                let id = match event.id().unwrap().parse::<usize>().ok() {
                    Some(id) => id,
                    None => return Some(Message::close(Reason::UnknownSubscription)),
                };

                let sender = match self.operations.get_mut(&id) {
                    Some(id) => id,
                    None => return Some(Message::close(Reason::UnknownSubscription)),
                };

                let payload = event.forwarding_payload().unwrap();

                if sender.send(payload).await.is_err() {
                    todo!("error")
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

    fn next(&mut self) -> impl Future<Output = Option<Next>> + '_ {
        let mut next_command = self.commands.next().fuse();
        let mut next_message = self.connection.receive().fuse();
        async move {
            // TODO: Handle termination of these streams appropriately...
            futures::select! {
                command = next_command => {
                    Some(Next::Command(command.expect("TODO: handle termination")))
                },
                message = next_message => {
                    Some(Next::Message(message.expect("TODO: handle termination")))
                },
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
        todo!()
    }
}

impl Event {
    fn forwarding_payload(self) -> Option<Value> {
        match self {
            Event::Next { id, payload } => Some(payload),
            Event::Error { id, payload } => Some(json!({"errors": payload})),
            _ => None,
        }
    }
}
