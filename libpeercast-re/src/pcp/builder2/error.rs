#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Target Not Found")]
    TargetNotFound,

    #[error("Invalid Payload")]
    InvalidPayload,
}
