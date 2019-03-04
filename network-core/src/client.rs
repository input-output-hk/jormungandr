//! Abstractions for the client-side network interface of a blockchain node.

pub mod block;
pub mod gossip;

mod error;

pub use error::{Error, ErrorKind};
