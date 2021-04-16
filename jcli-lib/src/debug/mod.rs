//! Debug tools for inspecting hex-encoded messages and blocks.
mod block;
mod message;
use hex::FromHexError;
use std::path::PathBuf;
#[cfg(feature = "structopt")]
use structopt::StructOpt;
use thiserror::Error;

#[cfg_attr(
    feature = "structopt",
    derive(StructOpt),
    structopt(rename_all = "kebab-case")
)]
/// Type for inspecting hex-encoded messages and blocks.
pub enum Debug {
    /// Decode hex-encoded message and display its content
    Message(message::Message),
    /// Decode hex-encoded block and display its content
    Block(block::Block),
}

#[derive(Debug, Error)]
/// Error types when inspecting hex-encoded messages and blocks.
pub enum Error {
    #[error("I/O Error")]
    Io(#[from] std::io::Error),
    #[error("invalid input file path '{path}'")]
    InputInvalid {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },
    #[error("hex encoding malformed")]
    HexMalformed(#[from] FromHexError),
    #[error("message malformed")]
    MessageMalformed(#[source] std::io::Error),
}

impl Debug {
    pub fn exec(self) -> Result<(), Error> {
        match self {
            Debug::Message(message) => message.exec(),
            Debug::Block(block) => block.exec(),
        }
    }
}
