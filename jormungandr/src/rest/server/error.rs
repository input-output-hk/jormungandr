use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to open server certificate file")]
    CertFileOpenFailed(#[source] io::Error),
    #[error("failed to parse server certificate file")]
    CertFileParsingFailed,
    #[error("server certificate file contains no certificates")]
    CertFileEmpty,
    #[error("failed to open server private key file")]
    PrivKeyFileOpenFailed(#[source] io::Error),
    #[error("failed to parse server private key file")]
    PrivKeyFileParsingFailed,
    #[error("server private key file should contain 1 key, contains {0}")]
    PrivKeyFileKeyCountInvalid(usize),
    #[error("failed to set server certificate")]
    SetCertFailed(#[source] rustls::TLSError),
    #[error("failed to bind the port")]
    BindFailed(#[source] io::Error),
    #[error("couldn't stop server, it's already stopped")]
    ServerAlreadyStopped,
    #[error("timeout during server stopping")]
    ServerStopTimeout,
    #[error("failed to stop server")]
    ServerStopFailed,
}
