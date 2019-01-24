//! Abstractions for the client-side network interface of a blockchain node.

use std::{error, fmt};

pub mod block;

/// Represents errors that can be returned by the node client implementation.
#[derive(Debug)]
pub enum Error {
    /// Error with protocol payload
    Format,
    /// An error with the protocol RPC call
    Rpc,
    // FIXME: add underlying error payload
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Format => write!(f, "malformed block received"),
            Error::Rpc => write!(f, "protocol error occurred"),
        }
    }
}
