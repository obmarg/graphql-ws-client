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
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
    use futures::StreamExt;
    use graphql_ws_client::CynicClientBuilder;

    let mut request = "ws://localhost:8000/graphql".into_client_request().unwrap();
    request.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        HeaderValue::from_str("graphql-transport-ws").unwrap(),
    );

    let (connection, _) = async_tungstenite::async_std::connect_async(request)
        .await
        .unwrap();

    println!("Connected");

    let (sink, stream) = connection.split();

    let mut client = CynicClientBuilder::new()
        .build(stream, sink, async_executors::AsyncStd)
        .await
        .unwrap();

    let mut stream = client.streaming_operation(build_query()).await.unwrap();
    println!("Running subscription");
    while let Some(item) = stream.next().await {
        println!("{:?}", item);
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
