// TODO: Start a server w/ async-graphql.
// Query that server...

use async_graphql::{EmptyMutation, Object, Schema, SimpleObject, Subscription, ID};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{extract::Extension, routing::post, Router};
use futures::{Stream, StreamExt};
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;

pub type BooksSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

pub struct SubscriptionServer {
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    port: u16,
    sender: Sender<BookChanged>,
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

        let schema = Schema::build(
            QueryRoot,
            EmptyMutation,
            SubscriptionRoot {
                channel: channel.clone(),
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

        SubscriptionServer {
            port,
            shutdown: Some(shutdown_sender),
            sender: channel,
        }
    }

    pub fn websocket_url(&self) -> String {
        format!("ws://localhost:{}/ws", self.port)
    }

    pub fn send(
        &self,
        change: BookChanged,
    ) -> Result<(), tokio::sync::broadcast::error::SendError<BookChanged>> {
        self.sender.send(change).map(|_| ())
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
}

#[Subscription]
impl SubscriptionRoot {
    async fn books(&self, _mutation_type: MutationType) -> impl Stream<Item = BookChanged> {
        println!("Subscription received");
        BroadcastStream::new(self.channel.subscribe()).filter_map(|r| async move { r.ok() })
    }
}
