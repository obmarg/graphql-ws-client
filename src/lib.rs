//! # graphql-ws-client
//!
//! graphql-ws-client implements asynchronous GraphQL-over-Websocket using the
//! [graphql-transport-ws protocol][protocol].  It tries to be websocket client,
//! graphql client _and_ async executor agnostic and provides built in support
//! for:
//!
//! - [Cynic][cynic] & [Graphql-Client][graphql-client] GraphQL clients.
//! - [async-tungstenite][async-tungstenite] & [ws-stream-wasm][ws-stream-wasm] Websocket Clients .
//! - The [async-std][async-std] & [tokio][tokio] async runtimes.
//!
//! If you'd like to use another client or runtime adding support should
//! hopefully be trivial.
//!
//! [protocol]: https://github.com/graphql/graphql-over-http/blob/main/rfcs/GraphQLOverWebSocket.md
//! [cynic]: https://cynic-rs.dev
//! [graphql-client]: https://github.com/graphql-rust/graphql-client
//! [async-tungstenite]: https://github.com/sdroege/async-tungstenite
//! [async-std]: https://async.rs/
//! [tokio]: https://tokio.rs/
//! [ws-stream-wasm]: https://github.com/najamelan/ws_stream_wasm

#![warn(missing_docs)]

mod error;
mod legacy;
mod logging;
mod protocol;

#[doc(hidden)]
#[path = "doc_utils.rs"]
pub mod __doc_utils;

pub mod graphql;

// TODO: next shouldn't be public really, and shouldn't allow missing_docs
#[allow(missing_docs)]
mod next;

#[cfg(feature = "ws_stream_wasm")]
/// Integration with the ws_stream_wasm library
pub mod ws_stream_wasm;

#[cfg(feature = "async-tungstenite")]
mod native;

#[allow(deprecated)]
pub use legacy::{
    client::{AsyncWebsocketClient, AsyncWebsocketClientBuilder, SubscriptionStream},
    websockets,
};

#[cfg(feature = "ws_stream_wasm")]
pub use legacy::wasm::{
    wasm_websocket_combined_split, FusedWasmWebsocketSink, WasmWebsocketMessage,
};

pub use next::*;

pub use error::Error;

/// A websocket client for the cynic graphql crate
#[cfg(feature = "cynic")]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type CynicClient<WsMessage> = AsyncWebsocketClient<WsMessage>;

/// A websocket client builder for the cynic graphql crate
#[cfg(feature = "cynic")]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type CynicClientBuilder = AsyncWebsocketClientBuilder;

/// A websocket client for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type GraphQLClientClient<WsMessage> = AsyncWebsocketClient<WsMessage>;

/// A websocket client builder for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
#[allow(deprecated)]
#[deprecated(
    since = "0.8.0-rc.1",
    note = "graphql-ws-client no longer needs client specific types.  Use the general purpose Client instead"
)]
pub type GraphQLClientClientBuilder = AsyncWebsocketClientBuilder;
