use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    BlockNotFound, // FIXME: add BlockId
    CannotIterate,
    BackendError(Box<dyn std::error::Error + Send + Sync>),
    Block0InFuture,
    BlockAlreadyPresent,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BlockNotFound => write!(f, "block not found"),
            Error::CannotIterate => write!(f, "cannot iterate between the 2 given blocks"),
            Error::BackendError(err) => write!(f, "{}", err),
            Error::Block0InFuture => write!(f, "block0 is in the future"),
            Error::BlockAlreadyPresent => write!(f, "Block already present in DB"),
        }
    }
}

impl error::Error for Error {}
