use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    BlockNotFound, // FIXME: add BlockId
    CannotIterate,
    BackendError(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BlockNotFound => write!(f, "block not found"),
            Error::CannotIterate => write!(f, "cannot iterate between the 2 given blocks"),
            Error::BackendError(_) => write!(f, "miscellaneous storage error"),
        }
    }
}

impl error::Error for Error {}
