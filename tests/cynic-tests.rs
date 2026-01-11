use std::{future::IntoFuture, time::Duration};

use assert_matches::assert_matches;
use cynic::StreamingOperation;
use futures_lite::{StreamExt, future};
use graphql_ws_client::Subscription;
use subscription_server::SubscriptionServer;
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
    let server = SubscriptionServer::start().await;

    let client_builder = server.client_builder().await;
    let (client, actor) = client_builder.await.unwrap();

    tokio::spawn(actor.into_future());

    let mut stream = client.subscribe(build_query()).await.unwrap();

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

    send_and_verify_updates(&server, &updates, &mut stream).await;
}

#[tokio::test]
async fn oneshot_operation_test() {
    let server = SubscriptionServer::start().await;

    let client_builder = server.client_builder().await;

    let mut stream = client_builder.subscribe(build_query()).await.unwrap();

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

    send_and_verify_updates(&server, &updates, &mut stream).await;
}

#[tokio::test]
async fn test_client_stop() {
    let server = SubscriptionServer::start().await;

    let client_builder = server.client_builder().await;
    let (client, actor) = client_builder.await.unwrap();

    tokio::spawn(actor.into_future());

    let mut stream = client.subscribe(build_query()).await.unwrap();

    sleep(Duration::from_millis(10)).await;

    let updates = [subscription_server::BookChanged {
        id: "123".into(),
        book: None,
    }];

    send_and_verify_updates(&server, &updates, &mut stream).await;

    assert_eq!(server.subscriber_count(), 1);

    client.stop(stream.id()).await.unwrap();

    sleep(Duration::from_millis(10)).await;

    assert_eq!(server.subscriber_count(), 0);

    assert!(stream.next().await.is_none());
}

#[tokio::test]
async fn test_subscription_stop() {
    let server = SubscriptionServer::start().await;

    let client_builder = server.client_builder().await;
    let (client, actor) = client_builder.await.unwrap();

    tokio::spawn(actor.into_future());

    let mut stream = client.subscribe(build_query()).await.unwrap();

    sleep(Duration::from_millis(10)).await;

    let updates = [subscription_server::BookChanged {
        id: "123".into(),
        book: None,
    }];

    send_and_verify_updates(&server, &updates, &mut stream).await;

    assert_eq!(server.subscriber_count(), 1);

    stream.stop();

    sleep(Duration::from_millis(10)).await;

    assert_eq!(server.subscriber_count(), 0);
}

#[tokio::test]
async fn test_subscription_stops_on_drop() {
    let server = SubscriptionServer::start().await;

    let client_builder = server.client_builder().await;
    let (client, actor) = client_builder.await.unwrap();

    tokio::spawn(actor.into_future());

    let mut stream = client.subscribe(build_query()).await.unwrap();

    sleep(Duration::from_millis(10)).await;

    let updates = [subscription_server::BookChanged {
        id: "123".into(),
        book: None,
    }];

    send_and_verify_updates(&server, &updates, &mut stream).await;

    assert_eq!(server.subscriber_count(), 1);

    drop(stream);

    sleep(Duration::from_millis(10)).await;

    assert_eq!(server.subscriber_count(), 0);
}

async fn send_and_verify_updates(
    server: &SubscriptionServer,
    updates: &[subscription_server::BookChanged],
    stream: &mut Subscription<StreamingOperation<BooksChangedSubscription, BooksChangedVariables>>,
) {
    future::zip(
        async {
            sleep(Duration::from_millis(10)).await;
            for update in updates {
                server.send(update.to_owned()).unwrap();
            }
        },
        async {
            let mut received_updates = Vec::new();
            for _ in updates {
                received_updates.push(stream.next().await.unwrap());
            }

            for (expected, update) in updates.iter().zip(received_updates) {
                let update = update.unwrap();
                assert_matches!(update.errors, None);
                let data = update.data.unwrap();
                assert_eq!(data.books.id.inner(), expected.id.0);
            }
        },
    )
    .await;
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
