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

mod client;
mod logging;
mod protocol;

pub mod graphql;
pub mod websockets;

// TODO: next shouldn't be public really, and shouldn't allow missing_docs
#[allow(missing_docs)]
pub mod next;

#[cfg(feature = "ws_stream_wasm")]
mod wasm;
#[cfg(feature = "ws_stream_wasm")]
pub use wasm::{wasm_websocket_combined_split, FusedWasmWebsocketSink, WasmWebsocketMessage};

#[cfg(feature = "ws_stream_wasm")]
/// Integration with the ws_stream_wasm library
pub mod ws_stream_wasm;

#[cfg(feature = "async-tungstenite")]
mod native;

pub use client::{AsyncWebsocketClient, AsyncWebsocketClientBuilder, Error, SubscriptionStream};

/// A websocket client for the cynic graphql crate
#[cfg(feature = "cynic")]
pub type CynicClient<WsMessage> = AsyncWebsocketClient<WsMessage>;
/// A websocket client builder for the cynic graphql crate
#[cfg(feature = "cynic")]
pub type CynicClientBuilder = AsyncWebsocketClientBuilder;

/// A websocket client for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
pub type GraphQLClientClient<WsMessage> = AsyncWebsocketClient<WsMessage>;
/// A websocket client builder for the graphql_client graphql crate
#[cfg(feature = "client-graphql-client")]
pub type GraphQLClientClientBuilder = AsyncWebsocketClientBuilder;
