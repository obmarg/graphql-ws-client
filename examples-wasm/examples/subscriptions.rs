//! An example of using subscriptions with `graphql-ws-client` and
//! `ws_stream_wasm`
//!
//! Talks to the the tide subscription example in `async-graphql`

use std::future::IntoFuture;

use graphql_ws_client::{next::ClientBuilder, ws_stream_wasm::Connection};

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
    use futures::StreamExt;
    use log::info;
    use wasm_bindgen::UnwrapThrowExt;

    console_log::init_with_level(log::Level::Info).expect("init logging");

    let ws_conn = ws_stream_wasm::WsMeta::connect(
        "ws://localhost:8000/graphql",
        Some(vec!["graphql-transport-ws"]),
    )
    .await
    .expect_throw("assume the connection succeeds");

    info!("Connected");

    let connection = Connection::new(ws_conn).await;

    let (mut client, actor) = ClientBuilder::new().build(connection).await.unwrap();
    wasm_bindgen_futures::spawn_local(actor.into_future());

    let mut stream = client.streaming_operation(build_query()).await.unwrap();
    info!("Running subscription");
    while let Some(item) = stream.next().await {
        info!("{:?}", item);
    }
}

fn build_query() -> cynic::StreamingOperation<BooksChangedSubscription> {
    use cynic::SubscriptionBuilder;

    BooksChangedSubscription::build(())
}
