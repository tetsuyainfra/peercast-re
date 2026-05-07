use thiserror::Error;

#[derive(Debug, Error)]
pub enum InfoParseError {
    #[error("Target not found")]
    TargetNotFound,
    // MissingField(&'static str),
    #[error("Unknown value type")]
    UnknownValueType,
}
