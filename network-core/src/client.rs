//! Abstractions for the client-side network interface of a blockchain node.

pub mod block;

mod error;

pub use error::{Error, ErrorKind};
