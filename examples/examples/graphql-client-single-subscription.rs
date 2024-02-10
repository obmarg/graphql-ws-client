//! An example of creating a connection and running a single subscription on it
//! using `graphql-client` and `async-tungstenite`
//!
//! Talks to the the tide subscription example in `async-graphql`

use futures::StreamExt;
use graphql_client::GraphQLQuery;
use graphql_ws_client::graphql::StreamingOperation;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "examples/graphql-client-subscription.graphql",
    schema_path = "../schemas/books.graphql",
    response_derives = "Debug"
)]
struct BooksChanged;

#[async_std::main]
async fn main() {
    use async_tungstenite::tungstenite::{client::IntoClientRequest, http::HeaderValue};
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

    let mut subscription = Client::build(connection)
        .subscribe(StreamingOperation::<BooksChanged>::new(
            books_changed::Variables,
        ))
        .await
        .unwrap();

    while let Some(item) = subscription.next().await {
        println!("{:?}", item);
    }
}
