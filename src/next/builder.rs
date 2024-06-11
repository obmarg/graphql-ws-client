use std::{
    future::{Future, IntoFuture},
    time::Duration,
};

use futures_lite::future;
use serde::Serialize;

use crate::{graphql::GraphqlOperation, logging::trace, protocol::Event, Error};

use super::{
    actor::ConnectionActor,
    connection::{Connection, Message, ObjectSafeConnection},
    keepalive::KeepAliveSettings,
    production_future::read_from_producer,
    Client, Subscription,
};

const DEFAULT_SUBSCRIPTION_BUFFER_SIZE: usize = 5;

pub mod next {
    use super::{
        run_startup, Client, Connection, ConnectionActor, Event, KeepAliveSettings, Message,
        Subscription, DEFAULT_SUBSCRIPTION_BUFFER_SIZE,
    };
    use crate::{graphql::GraphqlOperation, logging::trace, Error};
    use serde::Serialize;
    use std::{future::IntoFuture, time::Duration};

    /// Builder for Graphql over Websocket clients
    ///
    /// This can be used to configure the client prior to construction, but can also create
    /// subscriptions directly in the case where users only need to run one per connection.
    ///
    /// ```rust
    /// use graphql_ws_client::{Client, ClientBuilder};
    /// #
    /// # async fn example() -> Result<(), graphql_ws_client::Error> {
    /// # let connection = graphql_ws_client::__doc_utils::Conn;
    /// let (client, actor) = ClientBuilder::new().build(connection).await?;
    /// // or
    /// # let connection = graphql_ws_client::__doc_utils::Conn;
    /// let (client, actor) = Client::builder().build(connection).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[derive(Clone, Debug, Default)]
    pub struct ClientBuilder {
        payload: Option<serde_json::Value>,
        subscription_buffer_size: Option<usize>,
        keep_alive: KeepAliveSettings,
    }

    impl super::Client {
        /// Creates a ClientBuilder.
        ///
        /// Same as calling `ClientBuilder::new()`.
        ///
        /// ```rust
        /// use graphql_ws_client::{Client, ClientBuilder};
        /// #
        /// # async fn example() -> Result<(), graphql_ws_client::Error> {
        /// # let connection = graphql_ws_client::__doc_utils::Conn;
        /// let (client, actor) = Client::builder().build(connection).await?;
        /// # Ok(())
        /// # }
        /// ```
        pub fn builder() -> ClientBuilder {
            ClientBuilder::new()
        }
    }

    impl ClientBuilder {
        /// Creates a ClientBuilder.
        pub fn new() -> Self {
            Self::default()
        }

        /// Add payload to `connection_init`
        pub fn payload(self, payload: impl Serialize) -> Result<Self, Error> {
            Ok(Self {
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

        /// Sets the interval between keep alives.
        ///
        /// Any incoming messages automatically reset this interval so keep alives may not be sent
        /// on busy connections even if this is set.
        pub fn keep_alive_interval(mut self, new: Duration) -> Self {
            self.keep_alive.interval = Some(new);
            self
        }

        /// The number of keepalive retries before a connection is considered broken.
        ///
        /// This defaults to 3, but has no effect if `keep_alive_interval` is not called.
        pub fn keep_alive_retries(mut self, count: usize) -> Self {
            self.keep_alive.retries = count;
            self
        }

        /// Initialize a Client and use it to run a single subscription
        ///
        /// ```rust
        /// use graphql_ws_client::{Client, ClientBuilder};
        /// # async fn example() -> Result<(), graphql_ws_client::Error> {
        /// # let connection = graphql_ws_client::__doc_utils::Conn;
        /// # let subscription = graphql_ws_client::__doc_utils::Subscription;
        /// let stream = ClientBuilder::new().subscribe(connection, subscription).await?;
        /// // or
        /// # let connection = graphql_ws_client::__doc_utils::Conn;
        /// # let subscription = graphql_ws_client::__doc_utils::Subscription;
        /// let stream = Client::builder().subscribe(connection, subscription).await?;
        /// # Ok(())
        /// # }
        /// ```
        ///
        /// Note that this takes ownership of the client, so it cannot be
        /// used to run any more operations.
        ///
        /// If users want to run multiple operations on a connection they
        /// should `build` the `Client`.
        pub async fn subscribe<Conn, Operation>(
            self,
            connection: Conn,
            operation: Operation,
        ) -> Result<Subscription<Operation>, Error>
        where
            Conn: Connection + Send + 'static,
            Operation: GraphqlOperation + Unpin + Send + 'static,
        {
            let (client, actor) = self.build(connection).await?;

            let actor_future = actor.into_future();
            let subscribe_future = client.subscribe(operation);

            let (stream, actor_future) = run_startup(subscribe_future, actor_future).await?;

            Ok(stream.join(actor_future))
        }

        /// Constructs a Client
        ///
        /// Accepts an already built websocket connection, and returns the connection
        /// and a future that must be awaited somewhere - if the future is dropped the
        /// connection will also drop.
        pub async fn build<Conn>(
            self,
            mut connection: Conn,
        ) -> Result<(Client, ConnectionActor), Error>
        where
            Conn: Connection + Send + 'static,
        {
            let Self {
                payload,
                subscription_buffer_size,
                keep_alive,
            } = self;
            let subscription_buffer_size =
                subscription_buffer_size.unwrap_or(DEFAULT_SUBSCRIPTION_BUFFER_SIZE);

            connection.send(Message::init(payload)).await?;

            // Wait for ack before entering receiver loop:
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
                            // Pings can be sent at any time
                            Event::Ping { .. } => {
                                connection.send(Message::graphql_pong()).await?;
                            }
                            Event::Pong { .. } => {}
                            Event::ConnectionAck { .. } => {
                                // Handshake completed, ready to enter main receiver loop
                                trace!("connection_ack received, handshake completed");
                                break;
                            }
                            event => {
                                connection
                                    .send(Message::Close {
                                        code: Some(4950),
                                        reason: Some(
                                            "Unexpected message while waiting for ack".into(),
                                        ),
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

            let (command_sender, command_receiver) =
                async_channel::bounded(subscription_buffer_size);

            let actor = ConnectionActor::new(Box::new(connection), command_receiver, keep_alive);

            let client = Client::new_internal(command_sender, subscription_buffer_size);

            Ok((client, actor))
        }
    }
}

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
#[deprecated(since = "0.11.0", note = "use Client::builder() instead")]
pub struct ClientBuilder {
    payload: Option<serde_json::Value>,
    subscription_buffer_size: Option<usize>,
    connection: Box<dyn ObjectSafeConnection>,
    keep_alive: KeepAliveSettings,
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
    #[deprecated(since = "0.11.0", note = "use Client::builder() instead")]
    pub fn build<Conn>(connection: Conn) -> ClientBuilder
    where
        Conn: Connection + Send + 'static,
    {
        ClientBuilder {
            payload: None,
            subscription_buffer_size: None,
            connection: Box::new(connection),
            keep_alive: KeepAliveSettings::default(),
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

    /// Sets the interval between keep alives.
    ///
    /// Any incoming messages automatically reset this interval so keep alives may not be sent
    /// on busy connections even if this is set.
    pub fn keep_alive_interval(mut self, new: Duration) -> Self {
        self.keep_alive.interval = Some(new);
        self
    }

    /// The number of keepalive retries before a connection is considered broken.
    ///
    /// This defaults to 3, but has no effect if `keep_alive_interval` is not called.
    pub fn keep_alive_retries(mut self, count: usize) -> Self {
        self.keep_alive.retries = count;
        self
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

        let actor_future = actor.into_future();
        let subscribe_future = client.subscribe(op);

        let (stream, actor_future) = run_startup(subscribe_future, actor_future).await?;

        Ok(stream.join(actor_future))
    }
}

impl IntoFuture for ClientBuilder {
    type Output = Result<(Client, ConnectionActor), Error>;

    type IntoFuture = future::Boxed<Self::Output>;

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
            keep_alive,
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

        let (command_sender, command_receiver) = async_channel::bounded(5);

        let actor = ConnectionActor::new(connection, command_receiver, keep_alive);

        let client = Client::new_internal(command_sender, subscription_buffer_size.unwrap_or(5));

        Ok((client, actor))
    }
}

async fn run_startup<SubscribeFut, Operation>(
    subscribe: SubscribeFut,
    actor: future::Boxed<()>,
) -> Result<(Subscription<Operation>, future::Boxed<()>), Error>
where
    SubscribeFut: Future<Output = Result<Subscription<Operation>, Error>>,
    Operation: GraphqlOperation,
{
    match read_from_producer(subscribe, actor).await {
        Some((Ok(subscription), actor)) => Ok((subscription, actor)),
        Some((Err(err), _)) => Err(err),
        None => Err(Error::Unknown(
            "actor ended before subscription started".into(),
        )),
    }
}
