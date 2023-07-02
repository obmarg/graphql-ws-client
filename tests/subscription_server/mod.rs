// TODO: Start a server w/ async-graphql.
// Query that server...

use async_graphql::{EmptyMutation, Object, Schema, SimpleObject, Subscription, ID};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{extract::Extension, routing::post, Router, Server};
use futures_util::{Stream, StreamExt};
use tokio::sync::broadcast::Sender;
use tokio_stream::wrappers::BroadcastStream;

pub type BooksSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

pub async fn start(port: u16, channel: Sender<BookChanged>) {
    let schema = Schema::build(QueryRoot, EmptyMutation, SubscriptionRoot { channel }).finish();

    let app = Router::new()
        .route("/", post(graphql_handler))
        .route_service("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(Extension(schema));

    tokio::spawn(async move {
        Server::bind(&format!("127.0.0.1:{port}").parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });
}

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

pub struct SubscriptionRoot {
    channel: Sender<BookChanged>,
}

#[Subscription]
impl SubscriptionRoot {
    async fn books(&self) -> impl Stream<Item = BookChanged> {
        println!("Subscription received");
        BroadcastStream::new(self.channel.subscribe()).filter_map(|r| async move { r.ok() })
    }
}
