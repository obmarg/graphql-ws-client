//! An example of using subscriptions with `graphql-ws-client` and
//! `ws_stream_wasm`
//!
//! Talks to the the tide subscription example in `async-graphql`

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
    use graphql_ws_client::CynicClientBuilder;
    use log::info;
    use wasm_bindgen::UnwrapThrowExt;

    console_log::init_with_level(log::Level::Info).expect("init logging");

    let (ws, wsio) = ws_stream_wasm::WsMeta::connect(
        "ws://localhost:8000/graphql",
        Some(vec!["graphql-transport-ws"]),
    )
    .await
    .expect_throw("assume the connection succeeds");

    info!("Connected");

    let (sink, stream) = graphql_ws_client::wasm_websocket_combined_split(ws, wsio).await;

    let mut client = CynicClientBuilder::new()
        .build(stream, sink, async_executors::AsyncStd)
        .await
        .unwrap();

    let mut stream = client.streaming_operation(build_query()).await.unwrap();
    info!("Running subscription");
    while let Some(item) = stream.next().await {
        info!("{:?}", item);
    }
}

fn build_query() -> cynic::StreamingOperation<'static, BooksChangedSubscription> {
    use cynic::SubscriptionBuilder;

    BooksChangedSubscription::build(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn snapshot_test_menu_query() {
        // Running a snapshot test of the query building functionality as that gives us
        // a place to copy and paste the actual GQL we're using for running elsewhere,
        // and also helps ensure we don't change queries by mistake

        let query = build_query();

        insta::assert_snapshot!(query.query());
    }
}
