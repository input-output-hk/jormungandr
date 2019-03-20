use bech32::{Bech32 as Bech32Data, Error as Bech32Error, FromBase32, ToBase32};
use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt;
use std::result::Result as StdResult;

pub type Result<T> = StdResult<T, Error>;

pub trait Bech32 {
    const BECH32_HRP: &'static str;

    fn try_from_bech32_str(bech32_str: &str) -> Result<Self>
    where
        Self: Sized,
    {
        let bech32: Bech32Data = bech32_str.parse()?;
        if bech32.hrp() != Self::BECH32_HRP {
            return Err(Error::HrpInvalid {
                expected: Self::BECH32_HRP,
                actual: bech32.hrp().to_string(),
            });
        }
        let bytes = Vec::<u8>::from_base32(bech32.data())?;
        Self::try_from_bytes(&bytes)
    }

    fn try_from_bytes(bytes: &[u8]) -> Result<Self>
    where
        Self: Sized;

    fn to_bech32_str(&self) -> String {
        let data = self.to_bytes();
        Bech32Data::new(Self::BECH32_HRP.to_string(), data.to_base32())
            .unwrap_or_else(|e| panic!("Failed to build bech32: {}", e))
            .to_string()
    }

    fn to_bytes(&self) -> Cow<[u8]>;
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
