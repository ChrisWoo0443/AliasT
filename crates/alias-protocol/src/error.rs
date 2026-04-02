/// Protocol errors for message parsing and handling.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// The input is not valid JSON.
    #[error("invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// The JSON contains an unrecognized message type.
    #[error("unknown message type")]
    UnknownMessageType,
}
