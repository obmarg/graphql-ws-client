use std::future::IntoFuture;

use futures::{channel::mpsc, future::BoxFuture, FutureExt};
use serde::Serialize;

use crate::{graphql::GraphqlOperation, logging::trace, protocol::Event, Error};

use super::{
    actor::ConnectionActor,
    connection::{Connection, Message},
    Client, Subscription,
};

/// Builder for Graphql over Websocket clients
///
/// This can be used to configure the client prior to construction, but can also create
/// subscriptions directly in the case where users only need to run one per connection.
///
/// ```rust
/// use graphql_ws_client::Client;
/// use std::future::IntoFuture;
/// #
/// # async fn example() -> Result<(), graphql_ws_client::Error> {
/// # let connection = graphql_ws_client::__doc_utils::Conn;
/// let (client, actor) = Client::build(connection).await?;
/// # Ok(())
/// # }
/// ```
pub struct ClientBuilder {
    payload: Option<serde_json::Value>,
    subscription_buffer_size: Option<usize>,
    connection: Box<dyn Connection + Send>,
}

impl super::Client {
    /// Creates a ClientBuilder with the given connection.
    ///
    /// ```rust
    /// use graphql_ws_client::Client;
    /// use std::future::IntoFuture;
    /// # async fn example() -> Result<(), graphql_ws_client::Error> {
    /// # let connection = graphql_ws_client::__doc_utils::Conn;
    /// let (client, actor) = Client::build(connection).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build<Conn>(connection: Conn) -> ClientBuilder
    where
        Conn: Connection + Send + 'static,
    {
        ClientBuilder {
            payload: None,
            subscription_buffer_size: None,
            connection: Box::new(connection),
        }
    }
}

impl ClientBuilder {
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

    /// Sets the size of the incoming message buffer that subscriptions created by this client will
    /// use
    pub fn subscription_buffer_size(self, new: usize) -> Self {
        ClientBuilder {
            subscription_buffer_size: Some(new),
            ..self
        }
    }

    /// Initialise a Client and use it to run a single subscription
    ///
    /// ```rust
    /// use graphql_ws_client::Client;
    /// use std::future::IntoFuture;
    /// # async fn example() -> Result<(), graphql_ws_client::Error> {
    /// # let connection = graphql_ws_client::__doc_utils::Conn;
    /// # let subscription = graphql_ws_client::__doc_utils::Subscription;
    /// let stream = Client::build(connection).subscribe(subscription).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note that this takes ownership of the client, so it cannot be
    /// used to run any more operations.
    ///
    /// If users want to run mutliple operations on a connection they
    /// should use the `IntoFuture` impl to construct a `Client`
    pub async fn subscribe<'a, Operation>(
        self,
        op: Operation,
    ) -> Result<Subscription<Operation>, Error>
    where
        Operation: GraphqlOperation + Unpin + Send + 'static,
    {
        let (client, actor) = self.await?;

        let mut actor_future = actor.into_future().fuse();

        let subscribe_future = client.subscribe(op).fuse();
        futures::pin_mut!(subscribe_future);

        // Temporarily run actor_future while we start the subscription
        let stream = futures::select! {
            () = actor_future => {
                return Err(Error::Unknown("actor ended before subscription started".into()))
            },
            result = subscribe_future => {
                result?
            }
        };

        Ok(stream.join(actor_future))
    }
}

impl IntoFuture for ClientBuilder {
    type Output = Result<(Client, ConnectionActor), Error>;

    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.build())
    }
}

impl ClientBuilder {
    /// Constructs a Client
    ///
    /// Accepts an already built websocket connection, and returns the connection
    /// and a future that must be awaited somewhere - if the future is dropped the
    /// connection will also drop.
    pub async fn build(self) -> Result<(Client, ConnectionActor), Error> {
        let Self {
            payload,
            subscription_buffer_size,
            mut connection,
        } = self;

        connection.send(Message::init(payload)).await?;

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
                            connection
                                .send(Message::Close {
                                    code: Some(4950),
                                    reason: Some("Unexpected message while waiting for ack".into()),
                                })
                                .await
                                .ok();
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

        let client = Client::new_internal(command_sender, subscription_buffer_size.unwrap_or(5));

        Ok((client, actor))
    }
}
