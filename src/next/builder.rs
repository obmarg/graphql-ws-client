use std::{future::IntoFuture, time::Duration};

use futures_lite::{future, pin};
use serde::Serialize;

use crate::{graphql::GraphqlOperation, logging::trace, protocol::Event, Error};

use super::{
    actor::ConnectionActor,
    connection::{Connection, Message},
    keepalive::KeepAliveSettings,
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

        enum Either {
            ActorDied,
            Subscribed(Result<Subscription<Operation>, Error>),
        }

        let actor_future = actor.into_future();
        let subscribe_future = client.subscribe(op);

        let (stream, actor_future) = run_startup(subscribe_future, actor_future).await?;

        Ok(stream.join(actor_future.fuse()))
    }
}

impl IntoFuture for ClientBuilder {
    type Output = Result<(Client, ConnectionActor), Error>;

    type IntoFuture = future::Boxed<'static, Self::Output>;

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

#[pin_project::pin_project]
pub struct StartupFuture<SubscribeFut, Operation>
where
    SubscribeFut: Future<Output = Result<Subscription<Operation>, Error>>,
    Operation: GraphqlOperation,
{
    #[pin]
    subscribe: SubscribeFut,
    pub actor: Option<BoxFuture<'static, ()>>,
}

impl<SubscribeFut, Operation> Future for StartupFuture<SubscribeFut, Operation>
where
    SubscribeFut: Future<Output = Result<Subscription<Operation>, Error>>,
    Operation: GraphqlOperation,
{
    type Output = Result<(Subscription<Operation>, BoxFuture<'static, ()>), Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();

        let Some(actor) = this.actor else {
            return Poll::Ready(Err(Error::Unknown(
                "internal error: startup future polled twice".into(),
            )));
        };

        match this.subscribe.poll(cx) {
            Poll::Ready(Ok(subscription)) => {
                return Poll::Ready(Ok((subscription, this.actor.take().unwrap())))
            }
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Pending => {}
        }

        use futures_lite::future::FutureExt;
        if actor.poll(cx).is_ready() {
            return Poll::Ready(Err(Error::Unknown(
                "actor ended before subscription started".into(),
            )));
        }

        Poll::Pending
    }
}

async fn run_startup<SubscribeFut, Operation>(
    subscribe: SubscribeFut,
    actor: BoxFuture<'static, ()>,
) -> Result<(Subscription<Operation>, BoxFuture<'static, ()>), Error>
where
    SubscribeFut: Future<Output = Result<Subscription<Operation>, Error>>,
    Operation: GraphqlOperation,
{
    StartupFuture {
        subscribe,
        actor: Some(actor),
    }
    .await
}
