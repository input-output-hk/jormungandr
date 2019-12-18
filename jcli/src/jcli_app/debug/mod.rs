mod block;
mod message;
use hex::FromHexError;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Debug {
    /// Decode hex-encoded message and display its content
    Message(message::Message),
    /// Decode hex-encoded block and display its content
    Block(block::Block),
}

#[derive(Debug, Error)]
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
