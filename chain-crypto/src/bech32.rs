use bech32::{Bech32 as Bech32Data, Error as Bech32Error, FromBase32, ToBase32};
use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

pub trait Bech32 {
    const BECH32_HRP: &'static str;

    fn try_from_bech32_str(bech32_str: &str) -> Result<Self>
    where
        Self: Sized;

    fn to_bech32_str(&self) -> String;
}

pub fn to_bech32_from_bytes<B: Bech32>(bytes: &[u8]) -> String {
    Bech32Data::new(B::BECH32_HRP.to_string(), bytes.to_base32())
        .unwrap_or_else(|e| panic!("Failed to build bech32: {}", e))
        .to_string()
}

pub fn try_from_bech32_to_bytes<B: Bech32>(bech32_str: &str) -> Result<Vec<u8>> {
    let bech32: Bech32Data = bech32_str.parse()?;
    if bech32.hrp() != B::BECH32_HRP {
        return Err(Error::HrpInvalid {
            expected: B::BECH32_HRP,
            actual: bech32.hrp().to_string(),
        });
    }
    Vec::<u8>::from_base32(bech32.data()).map_err(Into::into)
}

#[derive(Debug)]
pub enum Error {
    Bech32Malformed(Bech32Error),
    HrpInvalid {
        expected: &'static str,
        actual: String,
    },
    DataInvalid(Box<StdError + 'static>),
}

impl Error {
    pub fn data_invalid(cause: impl StdError + 'static) -> Self {
        Error::DataInvalid(Box::new(cause))
    }
}

impl From<Bech32Error> for Error {
    fn from(error: Bech32Error) -> Self {
        Error::Bech32Malformed(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
        match self {
            Error::Bech32Malformed(_) => write!(f, "Failed to parse bech32, invalid data format"),
            Error::HrpInvalid { expected, actual } => write!(
                f,
                "Parsed bech32 has invalid HRP prefix '{}', expected '{}'",
                actual, expected
            ),
            Error::DataInvalid(_) => write!(f, "Failed to parse data decoded from bech32"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::Bech32Malformed(cause) => Some(cause),
            Error::DataInvalid(cause) => Some(&**cause),
            _ => None,
        }
    }
}
