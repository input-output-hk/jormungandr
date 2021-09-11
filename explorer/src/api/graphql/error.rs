use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("internal error (this shouldn't happen) {0}")]
    InternalError(String),
    #[error("internal error (this shouldn't happen)")]
    InternalDbError,
    #[error("resource not found {0}")]
    NotFound(String),
    #[error("feature not implemented yet")]
    Unimplemented,
    #[error("invalid argument {0}")]
    ArgumentError(String),
    #[error("invalud pagination cursor {0}")]
    InvalidCursor(String),
    #[error("invalid address {0}")]
    InvalidAddress(String),
}
