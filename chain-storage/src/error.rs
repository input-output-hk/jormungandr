use std::{error, fmt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    BlockNotFound, // FIXME: add BlockId
    CannotIterate,
    BackendError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BlockNotFound => write!(f, "block not found"),
            Error::CannotIterate => write!(f, "cannot iterate between the 2 given blocks"),
            Error::BackendError => write!(f, "miscellaneous storage error"),
        }
    }
}

impl error::Error for Error {}
