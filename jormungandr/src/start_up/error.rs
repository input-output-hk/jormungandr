use crate::{
    blockcfg, blockchain,
    diagnostic::DiagnosticError,
    explorer, network, secure,
    settings::{self, logging},
};
use chain_storage::Error as StorageError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("SQLite file")]
    SQLite,
    #[error("Block0")]
    Block0,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Unable to initialize the logger")]
    LoggingInitializationError(#[from] logging::Error),
    #[error("Error in the overall configuration of the node")]
    ConfigurationError(#[from] settings::Error),
    #[error("I/O Error with {reason}")]
    IO {
        #[source]
        source: io::Error,
        reason: ErrorKind,
    },
    #[error("Parsing error on {reason}")]
    ParseError {
        #[source]
        source: io::Error,
        reason: ErrorKind,
    },
    #[error("Block 0 mismatch. expecting hash: {expected} but got : {got}")]
    Block0Mismatch {
        expected: blockcfg::HeaderId,
        got: blockcfg::HeaderId,
    },
    #[error("Storage error")]
    StorageError(#[from] StorageError),
    #[error("Error while loading the legacy blockchain state")]
    Blockchain(#[from] blockchain::Error),
    #[error("Error in the genesis-block")]
    Block0(#[from] blockcfg::Block0Error),
    #[error("Error fetching the genesis block from the network")]
    FetchBlock0(#[from] network::FetchBlockError),
    #[error("Error while loading the blockchain from the network")]
    NetworkBootstrapError(#[from] network::BootstrapError),
    #[error("Error while loading the node's secrets.")]
    NodeSecrets(#[from] secure::NodeSecretFromFileError),
    #[error("Block 0 is set to start in the future")]
    Block0InFuture,
    #[error("Error while loading the explorer from storage")]
    ExplorerBootstrapError(#[from] explorer::error::Error),
    #[error("A service has terminated with an error")]
    ServiceTerminatedWithError,
    #[error("Unable to get system limits: {0}")]
    DiagnosticError(#[from] DiagnosticError),
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
            Error::Block0Mismatch { .. } => 7,
            Error::Block0InFuture => 7,
            Error::NodeSecrets { .. } => 8,
            Error::FetchBlock0 { .. } => 9,
            Error::NetworkBootstrapError { .. } => 10,
            Error::ExplorerBootstrapError { .. } => 11,
            Error::ServiceTerminatedWithError => 12,
            Error::DiagnosticError { .. } => 13,
        }
    }
}
