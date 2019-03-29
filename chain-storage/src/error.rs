use std::{error, fmt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    BlockNotFound, // FIXME: add BlockId
    CannotIterate,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BlockNotFound => write!(f, "block not found"),
            Error::CannotIterate => write!(f, "cannot iterate between the 2 given blocks"),
        }
    }
}

impl error::Error for Error {}
