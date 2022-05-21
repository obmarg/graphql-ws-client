use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{stream::SplitSink, Sink, Stream, StreamExt};
use pharos::{Observable, ObserveConfig};
use pin_project_lite::pin_project;
use ws_stream_wasm::{WsErr, WsEvent, WsMessage, WsMeta, WsStream};

use crate::websockets::WebsocketMessage;

/// Creates a new pair of sink and stream which operate on [`WasmWebsocketMessage`] instead of `WsMessage` and `WsEvent` separately.
pub async fn wasm_websocket_combined_split(
    mut ws_meta: WsMeta,
    ws_stream: WsStream,
) -> (
    impl Sink<WasmWebsocketMessage, Error = WsErr>,
    impl Stream<Item = Result<WasmWebsocketMessage, WsErr>>,
) {
    let event_stream = ws_meta.observe(ObserveConfig::default()).await.unwrap();

    let (sink, message_stream) = ws_stream.split();

    let message_stream = message_stream.map(|f| Ok::<_, WsErr>(WasmWebsocketMessage::WsMessage(f)));

    let event_stream = event_stream.map(|f| match f {
        WsEvent::WsErr(e) => Err(e),
        e => Ok(WasmWebsocketMessage::WsEvent(e)),
    });

    let stream = futures::stream::select(event_stream, message_stream);

    let sink = FusedWasmWebsocketSink::new(sink);

    (sink, stream)
}

#[derive(Debug)]
/// A WebSocket event abstraction to combine the differentiated WebSocket events and messages of `ws_stream_wasm`.
pub enum WasmWebsocketMessage {
    /// A WebSocket message, text or binary, was received.
    WsMessage(ws_stream_wasm::WsMessage),
    /// A WebSocket event other than a message was received.
    WsEvent(ws_stream_wasm::WsEvent),
}

impl WebsocketMessage for WasmWebsocketMessage {
    type Error = ws_stream_wasm::WsErr;

    fn new(text: String) -> Self {
        WasmWebsocketMessage::WsMessage(ws_stream_wasm::WsMessage::Text(text))
    }

    fn text(&self) -> Option<&str> {
        match self {
            WasmWebsocketMessage::WsMessage(ws_stream_wasm::WsMessage::Text(text)) => {
                Some(text.as_ref())
            }
            _ => None,
        }
    }

    fn error_message(&self) -> Option<String> {
        match self {
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::WsErr(error)) => {
                Some(error.to_string())
            }
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::Error) => {
                Some("An undisclosed error happened on the connection.".into())
            }
            _ => None,
        }
    }

    fn is_ping(&self) -> bool {
        false
    }

    fn is_pong(&self) -> bool {
        false
    }

    fn is_close(&self) -> bool {
        matches!(
            self,
            WasmWebsocketMessage::WsEvent(ws_stream_wasm::WsEvent::Closed(_))
        )
    }
}

pin_project! {
    /// Sink for the [`with`](super::SinkExt::with) method.
    #[must_use = "sinks do nothing unless polled"]
    pub struct FusedWasmWebsocketSink {
        #[pin]
        sink: SplitSink<WsStream, WsMessage>,
    }
}

impl FusedWasmWebsocketSink {
    fn new(sink: SplitSink<WsStream, WsMessage>) -> Self {
        Self { sink }
    }
}

impl Sink<WasmWebsocketMessage> for FusedWasmWebsocketSink {
    type Error = WsErr;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsErr>> {
        self.project().sink.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: WasmWebsocketMessage) -> Result<(), WsErr> {
        let this = self.project();
        this.sink.start_send(match item {
            WasmWebsocketMessage::WsMessage(message) => message,
            WasmWebsocketMessage::WsEvent(_) => unreachable!("Someone tried to send an event instead of a plain message. This is a bug as it should not be possible."),
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsErr>> {
        self.project().sink.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), WsErr>> {
        self.project().sink.poll_close(cx)
    }
}
