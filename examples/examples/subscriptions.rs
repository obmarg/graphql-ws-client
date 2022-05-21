//! An example of using subscriptions with `graphql-ws-client` and
//! `async-tungstenite`
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
    #[cfg(not(target_arch = "wasm32"))]
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen::UnwrapThrowExt;

    use futures::StreamExt;
    use graphql_ws_client::CynicClientBuilder;
    use log::info;

    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Info).expect("init logging");

    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::init();

    #[cfg(not(target_arch = "wasm32"))]
    let (sink, stream) = {
        let mut request = "ws://localhost:8000/graphql".into_client_request().unwrap();
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            HeaderValue::from_str("graphql-transport-ws").unwrap(),
        );
        let (connection, _) = async_tungstenite::async_std::connect_async(request)
            .await
            .unwrap();
        connection.split()
    };

    #[cfg(target_arch = "wasm32")]
    let (sink, stream) = {
        let (ws, wsio) = ws_stream_wasm::WsMeta::connect(
            "ws://localhost:8000/graphql",
            Some(vec!["graphql-transport-ws"]),
        )
        .await
        .expect_throw("assume the connection succeeds");
        graphql_ws_client::wasm_websocket_combined_split(ws, wsio).await
    };

    info!("Connected");

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

        insta::assert_snapshot!(query.query);
    }

    #[test]
    fn test_running_query() {
        let result = run_query();
        if result.errors.is_some() {
            assert_eq!(result.errors.unwrap().len(), 0);
        }
        insta::assert_debug_snapshot!(result.data);
    }
}
