//! # graphql-ws-client
//!
//! graphql-ws-client implements asynchronous GraphQL-over-Websocket using the
//! [graphql-transport-ws protocol][protocol].  It is websocket client, graphql
//! client _and_ async runtime agnostic.  Built in support is provided for:
//!
//! - [Cynic][cynic] & [Graphql-Client][graphql-client] GraphQL clients.
//! - [async-tungstenite][async-tungstenite] & [ws-stream-wasm][ws-stream-wasm] Websocket Clients .
//! - Any async runtime.
//!
//! If you'd like to use another client or adding support should be trivial.
//!
//! ```rust
//! use graphql_ws_client::Client;
//! use std::future::IntoFuture;
//! use futures::StreamExt;
//! # async fn example() -> Result<(), graphql_ws_client::Error> {
//! # let connection = graphql_ws_client::__doc_utils::Conn;
//! # let subscription = graphql_ws_client::__doc_utils::Subscription;
//!
//! let mut stream = Client::build(connection).subscribe(subscription).await?;
//!
//! while let Some(response) = stream.next().await {
//!     // Do something with response
//! }
//! # Ok(())
//! # }
//! ```
//!
//! See the [examples][examples] for more thorough usage details.
//!
//! [protocol]: https://github.com/graphql/graphql-over-http/blob/main/rfcs/GraphQLOverWebSocket.md
//! [cynic]: https://cynic-rs.dev
//! [graphql-client]: https://github.com/graphql-rust/graphql-client
//! [async-tungstenite]: https://github.com/sdroege/async-tungstenite
//! [ws-stream-wasm]: https://github.com/najamelan/ws_stream_wasm
//! [examples]: https://github.com/obmarg/graphql-ws-client/tree/main/examples/examples

#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

mod error;
mod logging;
mod protocol;

#[doc(hidden)]
pub mod legacy;

#[doc(hidden)]
#[path = "doc_utils.rs"]
pub mod __doc_utils;

pub mod graphql;

mod next;

#[cfg(feature = "ws_stream_wasm")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws_stream_wasm")))]
/// Integration with the ws_stream_wasm library
pub mod ws_stream_wasm;

#[cfg(any(feature = "async-tungstenite", feature = "tungstenite"))]
mod native;

#[allow(deprecated)]
pub use legacy::{
    client::{AsyncWebsocketClient, AsyncWebsocketClientBuilder, SubscriptionStream},
    websockets,
};

#[cfg(feature = "ws_stream_wasm")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws_stream_wasm")))]
#[allow(deprecated)]
pub use legacy::wasm::{
    wasm_websocket_combined_split, FusedWasmWebsocketSink, WasmWebsocketMessage,
};

pub use next::*;

pub use error::Error;

/// A websocket client for the cynic graphql crate
#[cfg(feature = "cynic")]
#[cfg_attr(docsrs, doc(cfg(feature = "cynic")))]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type CynicClient<WsMessage> = AsyncWebsocketClient<WsMessage>;

/// A websocket client builder for the cynic graphql crate
#[cfg(feature = "cynic")]
#[cfg_attr(docsrs, doc(cfg(feature = "cynic")))]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type CynicClientBuilder = AsyncWebsocketClientBuilder;

/// A websocket client for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client-graphql-client")))]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type GraphQLClientClient<WsMessage> = AsyncWebsocketClient<WsMessage>;

/// A websocket client builder for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
#[cfg_attr(docsrs, doc(cfg(feature = "client-graphql-client")))]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type GraphQLClientClientBuilder = AsyncWebsocketClientBuilder;
