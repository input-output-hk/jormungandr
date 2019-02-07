use std::{error, fmt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    BlockNotFound, // FIXME: add BlockId
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::BlockNotFound => write!(f, "block not found"),
        }
    }
}

impl error::Error for Error {}
