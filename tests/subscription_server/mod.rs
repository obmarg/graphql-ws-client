// TODO: Start a server w/ async-graphql.
// Query that server...

use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_graphql::{EmptyMutation, ID, Object, Schema, SimpleObject, Subscription};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{Router, extract::Extension, http::HeaderValue, routing::post};
use futures_lite::{Stream, StreamExt};
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;
use tungstenite_0_27::client::IntoClientRequest;

pub type BooksSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

pub struct SubscriptionServer {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    port: u16,
    sender: Sender<BookChanged>,
    subscriber_count: Arc<AtomicUsize>,
}

impl Drop for SubscriptionServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            shutdown.send(()).ok();
        }
    }
}

impl SubscriptionServer {
    pub async fn start() -> SubscriptionServer {
        let (channel, _) = tokio::sync::broadcast::channel(16);
        let subscriber_count = Arc::new(AtomicUsize::new(0));

        let schema = Schema::build(
            QueryRoot,
            EmptyMutation,
            SubscriptionRoot {
                channel: channel.clone(),
                subscriber_count: Arc::clone(&subscriber_count),
            },
        )
        .finish();

        let app = Router::new()
            .route("/", post(graphql_handler))
            .route_service("/ws", GraphQLSubscription::new(schema.clone()))
            .layer(Extension(schema));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let (shutdown_sender, shutdown_receiver) = tokio::sync::oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, app.with_state(()))
                .with_graceful_shutdown(async move {
                    shutdown_receiver.await.ok();
                })
                .await
                .unwrap();
        });

        tokio::time::sleep(Duration::from_millis(20)).await;

        SubscriptionServer {
            port,
            shutdown: Some(shutdown_sender),
            sender: channel,
            subscriber_count,
        }
    }

    pub fn websocket_url(&self) -> String {
        format!("ws://localhost:{}/ws", self.port)
    }

    #[allow(unused)]
    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::Relaxed)
    }

    pub fn send(
        &self,
        change: BookChanged,
    ) -> Result<(), tokio::sync::broadcast::error::SendError<BookChanged>> {
        self.sender.send(change).map(|_| ())
    }

    #[allow(unused)]
    pub async fn client_builder(&self) -> graphql_ws_client::ClientBuilder {
        let mut request = self.websocket_url().into_client_request().unwrap();
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("graphql-transport-ws").unwrap(),
        );

        let (connection, _) = async_tungstenite::tokio::connect_async(request)
            .await
            .unwrap();

        println!("Connected");

        graphql_ws_client::Client::build(connection)
    }
}

#[axum_macros::debug_handler]
async fn graphql_handler(schema: Extension<BooksSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

#[derive(SimpleObject, Debug, Clone)]
pub struct Book {
    pub id: ID,
    pub name: String,
    pub author: String,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    pub async fn id(&self) -> ID {
        "123".into()
    }
}

#[derive(Clone, Debug, SimpleObject)]
pub struct BookChanged {
    pub book: Option<Book>,
    pub id: ID,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, async_graphql::Enum)]
enum MutationType {
    Created,
    Deleted,
}

pub struct SubscriptionRoot {
    channel: Sender<BookChanged>,
    subscriber_count: Arc<AtomicUsize>,
}

#[Subscription]
impl SubscriptionRoot {
    async fn books(&self, _mutation_type: MutationType) -> impl Stream<Item = BookChanged> {
        println!("Subscription received");
        self.subscriber_count.fetch_add(1, Ordering::Relaxed);
        TrackedBroadcastStream {
            inner: BroadcastStream::new(self.channel.subscribe()).filter_map(Result::ok),
            count: Arc::clone(&self.subscriber_count),
        }
    }
}

#[pin_project::pin_project(PinnedDrop)]
pub struct TrackedBroadcastStream<T> {
    #[pin]
    inner: T,
    count: Arc<AtomicUsize>,
}

impl<T> Stream for TrackedBroadcastStream<T>
where
    T: Stream,
{
    type Item = <T as Stream>::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

#[pin_project::pinned_drop]
impl<T> PinnedDrop for TrackedBroadcastStream<T> {
    fn drop(self: Pin<&mut Self>) {
        self.project().count.fetch_sub(1, Ordering::Relaxed);
    }
}
