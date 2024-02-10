#[derive(thiserror::Error, Debug)]
/// Error type
pub enum Error {
    /// Unknown error
    #[error("unknown: {0}")]
    Unknown(String),
    /// Custom error
    #[error("{0}: {1}")]
    Custom(String, String),
    /// Unexpected close frame
    #[error("got close frame. code: {0}, reason: {1}")]
    Close(u16, String),
    /// Decoding / parsing error
    #[error("message decode error, reason: {0}")]
    Decode(String),
    /// Serializing error
    #[error("couldn't serialize message, reason: {0}")]
    Serializing(String),
    /// Sending error
    #[error("message sending error, reason: {0}")]
    Send(String),
    /// Futures spawn error
    #[error("futures spawn error, reason: {0}")]
    SpawnHandle(String),
    /// Sender shutdown error
    #[error("sender shutdown error, reason: {0}")]
    SenderShutdown(String),
}
