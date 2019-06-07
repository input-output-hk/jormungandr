use crate::{
    blockcfg, blockchain, network, secure,
    settings::{self, logging},
};
use chain_storage::error::Error as StorageError;
use std::io;

custom_error! {pub ErrorKind
   SQLite = "SQLite file",
   Block0 = "Block0"
}

custom_error! {pub Error
    LoggingInitializationError { source: logging::Error } = "Unable to initialize the logger",
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
            Error::LoggingInitializationError { .. } => 1,
            Error::ConfigurationError { .. } => 2,
            Error::IO { .. } => 3,
            Error::ParseError { .. } => 4,
            Error::StorageError { .. } => 5,
            Error::Blockchain { .. } => 6,
            Error::Block0 { .. } => 7,
            Error::NodeSecrets { .. } => 8,
            Error::FetchBlock0 { .. } => 9,
        }
    }
}
