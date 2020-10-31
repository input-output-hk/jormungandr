use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Internal Error: `{0}`")]
    InternalError(String),
    #[error("Not Found: `{0}`")]
    NotFound(String),
    #[error("Unimplemented `{0}`")]
    Unimplemented(String),
    #[error("Argument Error `{0}`")]
    ArgumentError(String),
    #[error("Invalid Pagination cursor `{0}`")]
    InvalidCursor(String),
    #[error("Invalid address `{0}`")]
    InvalidAddress(String),
}
