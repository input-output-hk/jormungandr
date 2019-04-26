use crate::{blockcfg, blockchain, network, secure, settings};
use chain_storage::error::Error as StorageError;
use std::io;

custom_error! {pub ErrorKind
   SQLite = "SQLite file",
   Block0 = "Block0"
}

custom_error! {pub Error
    LoggingInitializationError = "Unable to initialize the logger",
    ConfigurationError{source: settings::Error} = "Error in the overall configuration of the node",
    IO{source: io::Error, reason: ErrorKind} = "I/O Error with {reason}",
    ParseError{ source: io::Error, reason: ErrorKind} = "Parsing error on {reason}",
    StorageError { source: StorageError } = "Storage error",
    Blockchain { source: blockchain::LoadError } = "Error while loading the blockchain state",
    Block0 { source: blockcfg::Block0Error } = "Error in the genesis-block",
    FetchBlock0 { source: network::FetchBlockError } = "Error fetching the genesis block from the network",
    NodeSecrets { source: secure::NodeSecretFromFileError} = "Error while loading the node's secrets."
}

impl Error {
    #[inline]
    pub fn code(&self) -> i32 {
        match self {
            Error::LoggingInitializationError => 1,
            Error::ConfigurationError { source: _ } => 2,
            Error::IO {
                source: _,
                reason: _,
            } => 3,
            Error::ParseError {
                source: _,
                reason: _,
            } => 4,
            Error::StorageError { source: _ } => 5,
            Error::Blockchain { source: _ } => 6,
            Error::Block0 { source: _ } => 7,
            Error::NodeSecrets { source: _ } => 8,
            Error::FetchBlock0 { .. } => 9,
        }
    }
}
