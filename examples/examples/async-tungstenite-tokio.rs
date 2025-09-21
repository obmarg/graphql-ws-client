//! An example of using subscriptions with `graphql-ws-client` and
//! `async-tungstenite`
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

#[tokio::main]
async fn main() {
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
    use futures::StreamExt;
    use graphql_ws_client::Client;

    let mut request = "ws://localhost:8000/graphql".into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );

    let (connection, _) = async_tungstenite::tokio::connect_async(request)
        .await
        .unwrap();

    println!("Connected");

    let (client, actor) = Client::build(connection).await.unwrap();
    tokio::spawn(actor.into_future());

    let mut stream = client.subscribe(build_query()).await.unwrap();
    println!("Running subscription apparently?");
    while let Some(item) = stream.next().await {
        println!("{item:?}");
    }
}

fn build_query() -> cynic::StreamingOperation<BooksChangedSubscription> {
    use cynic::SubscriptionBuilder;

    BooksChangedSubscription::build(())
}
