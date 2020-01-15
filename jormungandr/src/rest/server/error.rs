use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to load PKCS12 identity file")]
    Pkcs12LoadFailed(#[source] io::Error),
    #[error("invalid PKCS12 identity file")]
    Pkcs12Invalid(#[source] native_tls::Error),
    #[error("failed to bind the port")]
    BindFailed(#[source] io::Error),
    #[error("couldn't stop server, it's already stopped")]
    ServerAlreadyStopped,
    #[error("timeout during server stopping")]
    ServerStopTimeout,
    #[error("failed to stop server")]
    ServerStopFailed,
}
