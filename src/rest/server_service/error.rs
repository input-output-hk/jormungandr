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
        let error_name = match *self {
            Error::Pkcs12LoadFailed(_) => "Pkcs12LoadFailed",
            Error::Pkcs12Invalid(_) => "Pkcs12Invalid",
            Error::BindFailed(_) => "BindFailed",
            Error::ServerAlreadyStopped => "ServerAlreadyStopped",
            Error::ServerStopTimeout => "ServerStopTimeout ",
            Error::ServerStopFailed => "ServerStopFailed ",
        };
        write!(f, "Server service error: {}", error_name)?;
        if let Some(cause) = self.source() {
            write!(f, "caused by {}", cause)?
        }
        Ok(())
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
