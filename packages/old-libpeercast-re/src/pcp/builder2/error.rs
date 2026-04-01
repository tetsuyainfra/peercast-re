#[derive(Debug, thiserror::Error)]
pub enum InfoParseError {
    #[error("Target Not Found")]
    TargetNotFound,

    #[error("Invalid Payload")]
    InvalidPayload,

    #[error("Unknown Type")]
    UnknownType,
}
