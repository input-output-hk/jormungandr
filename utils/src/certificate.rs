use bech32::{Bech32, FromBase32, ToBase32};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property::Serialize;
use chain_impl_mockchain::certificate::Certificate;
use std::error::Error as StdError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    Bech32Error(bech32::Error),
    HrpInvalid {
        expected: &'static str,
        actual: String,
    },
    ReadError(ReadError),
}

impl From<bech32::Error> for Error {
    fn from(error: bech32::Error) -> Self {
        Error::Bech32Error(error)
    }
}

impl From<ReadError> for Error {
    fn from(error: ReadError) -> Self {
        Error::ReadError(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Error::Bech32Error(_) => write!(f, "Failed to parse bech32, invalid format"),
            Error::ReadError(_) => write!(f, "Failed read bech32"),
            Error::HrpInvalid { expected, actual } => write!(
                f,
                "Parsed bech32 has invalid HRP prefix '{}', expected '{}'",
                expected, actual
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Bech32Error(cause) => Some(cause),
            Error::ReadError(cause) => Some(cause),
            _ => None,
        }
    }
}

pub fn serialize_to_bech32(cert: &Certificate) -> Result<Bech32, Error> {
    let bytes = cert.serialize_as_vec().unwrap();
    Bech32::new("cert".to_string(), bytes.to_base32()).map_err(Into::into)
}

pub fn deserialize_from_bech32(bech32_str: &str) -> Result<Certificate, Error> {
    let bech32: Bech32 = bech32_str.parse()?;
    if bech32.hrp() != "cert" {
        return Err(Error::HrpInvalid {
            expected: "cert",
            actual: bech32.hrp().to_string(),
        });
    }
    let bytes = Vec::<u8>::from_base32(bech32.data())?;
    let mut buf = ReadBuf::from(&bytes);
    Certificate::read(&mut buf).map_err(Into::into)
}
