//! An example of running a multiple subscriptions on a single connection
//! using `cynic`, `async-tungstenite` & the `async_std`
//! executor.
//!
//! Talks to the the tide subscription example in `async-graphql`

use std::future::IntoFuture;

mod schema {
    cynic::use_schema!("../schemas/books.graphql");
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "../schemas/books.graphql", graphql_type = "Book")]
#[allow(dead_code)]
struct Book {
    id: String,
    name: String,
    author: String,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(schema_path = "../schemas/books.graphql", graphql_type = "BookChanged")]
#[allow(dead_code)]
struct BookChanged {
    id: cynic::Id,
    book: Option<Book>,
}

#[derive(cynic::QueryFragment, Debug)]
#[cynic(
    schema_path = "../schemas/books.graphql",
    graphql_type = "SubscriptionRoot"
)]
#[allow(dead_code)]
struct BooksChangedSubscription {
    books: BookChanged,
}

#[async_std::main]
async fn main() {
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
    use futures::StreamExt;
    use graphql_ws_client::Client;

    let mut request = "ws://localhost:8000/graphql".into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );

    let (connection, _) = async_tungstenite::async_std::connect_async(request)
        .await
        .unwrap();

    println!("Connected");

    let (mut client, actor) = Client::build(connection).await.unwrap();
    async_std::task::spawn(actor.into_future());

    // In reality you'd probably want to different subscriptions, but for the sake of this example
    // these are the same subscriptions
    let mut first_subscription = client.subscribe(build_query()).await.unwrap();
    let mut second_subscription = client.subscribe(build_query()).await.unwrap();

    futures::join!(
        async move {
            while let Some(item) = first_subscription.next().await {
                println!("{:?}", item);
            }
        },
        async move {
            while let Some(item) = second_subscription.next().await {
                println!("{:?}", item);
            }
        }
    );
}

fn build_query() -> cynic::StreamingOperation<BooksChangedSubscription> {
    use cynic::SubscriptionBuilder;

    BooksChangedSubscription::build(())
}
