use native_tls::Error as TlsError;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    Pkcs12LoadFailed(IoError),
    Pkcs12Invalid(TlsError),
    BindFailed(IoError),
    ServerAlreadyStopped,
    ServerStopTimeout,
    ServerStopFailed,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Pkcs12LoadFailed(_) => write!(f, "Failed to load PKCS12 identity file"),
            Error::Pkcs12Invalid(_) => write!(f, "Invalid PKCS12 identity file"),
            Error::BindFailed(_) => write!(f, "Failed to bind the port"),
            Error::ServerAlreadyStopped => write!(f, "Couldn't stop server, it's already stopped"),
            Error::ServerStopTimeout => write!(f, "Timeout during server stopping"),
            Error::ServerStopFailed => write!(f, "Failed to stop server"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            Error::Pkcs12LoadFailed(ref cause) => Some(cause),
            Error::Pkcs12Invalid(ref cause) => Some(cause),
            Error::BindFailed(ref cause) => Some(cause),
            _ => None,
        }
    }
}
