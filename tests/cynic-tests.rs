#![cfg(feature = "cynic")]

use std::time::Duration;

use assert_matches::assert_matches;
use tokio::time::sleep;

mod subscription_server;

mod schema {
    cynic::use_schema!("schemas/books.graphql");
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "schemas/books.graphql")]
#[allow(dead_code)]
struct Book {
    id: String,
    name: String,
    author: String,
}

#[derive(cynic::QueryFragment, Debug, Clone)]
#[cynic(schema_path = "schemas/books.graphql")]
#[allow(dead_code)]
struct BookChanged {
    id: cynic::Id,
    book: Option<Book>,
}

#[derive(cynic::QueryVariables)]
struct BooksChangedVariables {
    mutation_type: MutationType,
}

#[derive(cynic::Enum)]
#[cynic(schema_path = "schemas/books.graphql")]
enum MutationType {
    Created,
    Deleted,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema_path = "schemas/books.graphql",
    graphql_type = "SubscriptionRoot",
    variables = "BooksChangedVariables"
)]
#[allow(dead_code)]
struct BooksChangedSubscription {
    #[arguments(mutationType: $mutation_type)]
    books: BookChanged,
}

#[tokio::test]
async fn main_test() {
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
    use futures::StreamExt;
    use graphql_ws_client::CynicClientBuilder;

    let (channel, _) = tokio::sync::broadcast::channel(10);

    subscription_server::start(57432, channel.clone()).await;

    sleep(Duration::from_millis(20)).await;

    let mut request = "ws://localhost:57432/ws".into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );

    let (connection, _) = async_tungstenite::tokio::connect_async(request)
        .await
        .unwrap();

    println!("Connected");

    let (sink, stream) = connection.split();

    let mut client = CynicClientBuilder::new()
        .build(stream, sink, TokioSpawner::current())
        .await
        .unwrap();

    let stream = client.streaming_operation(build_query()).await.unwrap();

    sleep(Duration::from_millis(100)).await;

    let updates = [
        subscription_server::BookChanged {
            id: "123".into(),
            book: None,
        },
        subscription_server::BookChanged {
            id: "456".into(),
            book: None,
        },
        subscription_server::BookChanged {
            id: "789".into(),
            book: None,
        },
    ];

    futures::join!(
        async {
            for update in &updates {
                channel.send(update.to_owned()).unwrap();
            }
        },
        async {
            let received_updates = stream.take(updates.len()).collect::<Vec<_>>().await;

            for (expected, update) in updates.iter().zip(received_updates) {
                let update = update.unwrap();
                assert_matches!(update.errors, None);
                let data = update.data.unwrap();
                assert_eq!(data.books.id.inner(), expected.id.0);
            }
        }
    );
}

fn build_query() -> cynic::StreamingOperation<BooksChangedSubscription, BooksChangedVariables> {
    use cynic::SubscriptionBuilder;

    BooksChangedSubscription::build(BooksChangedVariables {
        mutation_type: MutationType::Created,
    })
}

impl PartialEq<subscription_server::Book> for Book {
    fn eq(&self, other: &subscription_server::Book) -> bool {
        self.id == other.id.0 && self.name == other.name && self.author == other.author
    }
}

pub struct TokioSpawner(tokio::runtime::Handle);

impl TokioSpawner {
    pub fn new(handle: tokio::runtime::Handle) -> Self {
        TokioSpawner(handle)
    }

    pub fn current() -> Self {
        TokioSpawner::new(tokio::runtime::Handle::current())
    }
}

impl futures::task::Spawn for TokioSpawner {
    fn spawn_obj(
        &self,
        obj: futures::task::FutureObj<'static, ()>,
    ) -> Result<(), futures::task::SpawnError> {
        self.0.spawn(obj);
        Ok(())
    }
}
