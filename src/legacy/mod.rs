#![allow(deprecated)]

pub mod client;
#[cfg(feature = "ws_stream_wasm")]
pub mod wasm;
pub mod websockets;
