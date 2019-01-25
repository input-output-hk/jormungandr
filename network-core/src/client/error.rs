use std::{error, fmt};

/// Represents errors that can be returned by the node client implementation.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    source: Box<dyn error::Error + Send + Sync>,
}

/// A list of general causes of client request errors.
///
/// This list is intended to grow over time and it is not recommended to
/// exhaustively match against it.
#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
    /// Error with protocol payload
    Format,
    /// An error with the protocol RPC call
    Rpc,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, source: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            kind,
            source: source.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Format => write!(f, "malformed payload received"),
            ErrorKind::Rpc => write!(f, "protocol error"),
        }
    }
}
