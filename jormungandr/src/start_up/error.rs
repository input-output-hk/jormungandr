use crate::{
    blockcfg, blockchain,
    blockchain::StorageError,
    diagnostic::DiagnosticError,
    explorer, network, secure,
    settings::{self, logging},
};
use chain_core::property::ReadError;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ErrorKind {
    #[error("block storage")]
    BlockStorage,
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
    Io {
        #[source]
        source: io::Error,
        reason: ErrorKind,
    },
    #[error("Parsing error on {reason}")]
    ParseError {
        #[source]
        source: ReadError,
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
    Blockchain(#[from] Box<blockchain::Error>),
    #[error("Error in the genesis-block")]
    Block0(#[from] blockcfg::Block0Error),
    #[error("Error fetching the genesis block from the network")]
    FetchBlock0(#[from] network::FetchBlockError),
    #[error("Error while loading the blockchain from the network")]
    NetworkBootstrapError(#[source] network::BootstrapError),
    #[error("Error while loading the node's secrets.")]
    NodeSecrets(#[from] secure::NodeSecretFromFileError),
    #[error("Block 0 is set to start in the future")]
    Block0InFuture,
    #[error("Error while loading the explorer from storage")]
    ExplorerBootstrapError(#[from] explorer::error::ExplorerError),
    #[error("A service has terminated with an error")]
    ServiceTerminatedWithError(#[from] crate::utils::task::ServiceError),
    #[error("Unable to get system limits: {0}")]
    DiagnosticError(#[from] DiagnosticError),
    #[error("Interrupted by the user")]
    Interrupted,
}

impl From<network::BootstrapError> for Error {
    fn from(error: network::BootstrapError) -> Error {
        match error {
            network::BootstrapError::Interrupted => Error::Interrupted,
            error => Error::NetworkBootstrapError(error),
        }
    }
}

impl Error {
    #[inline]
    pub fn code(&self) -> i32 {
        match self {
            Error::Interrupted => 0,
            Error::LoggingInitializationError { .. } => 1,
            Error::ConfigurationError { .. } => 2,
            Error::Io { .. } => 3,
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
            Error::ServiceTerminatedWithError { .. } => 12,
            Error::DiagnosticError { .. } => 13,
        }
    }
}
