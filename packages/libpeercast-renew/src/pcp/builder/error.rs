pub enum InfoParseError {
    TargetNotFound,
    InvalidId,
    MissingField(&'static str),
    InvalidField(&'static str),
}
