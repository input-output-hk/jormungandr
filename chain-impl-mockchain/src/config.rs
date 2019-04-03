use crate::block::ConsensusVersion;
use crate::message::initial::{Tag, TagPayload};
use chain_addr::Discrimination;
use num_traits::FromPrimitive;
use std::str::FromStr;

/// Seconds elapsed since 1-Jan-1970 (unix time)
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Block0Date(pub u64);

/// Possible errors
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    InvalidTag,
    SizeInvalid,
    StructureInvalid,
    UnknownString(String),
}

pub trait ConfigParam: Clone + Eq + PartialEq {
    const TAG: Tag;
    const NAME: &'static str;

    fn to_payload(&self) -> TagPayload;

    fn from_payload(payload: &TagPayload) -> Result<Self, Error>;

    fn to_string(&self) -> String;
    fn from_string(s: &str) -> Result<Self, Error>;
}

impl ConfigParam for Block0Date {
    const TAG: Tag = Tag::unchecked_new(1);
    const NAME: &'static str = "block0-date";

    fn to_payload(&self) -> TagPayload {
        let mut out = Vec::new();
        out.extend_from_slice(&self.0.to_be_bytes());
        out
    }

    fn from_payload(payload: &TagPayload) -> Result<Self, Error> {
        if payload.len() != 8 {
            return Err(Error::SizeInvalid);
        };
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&payload);
        let v = u64::from_be_bytes(bytes);
        Ok(Block0Date(v))
    }

    fn to_string(&self) -> String {
        format!("{}", self.0).to_string()
    }
    fn from_string(s: &str) -> Result<Self, Error> {
        let v = u64::from_str(s).map_err(|_| Error::UnknownString(s.to_string()))?;
        Ok(Block0Date(v))
    }
}

const VAL_PROD: u8 = 1;
const VAL_TEST: u8 = 2;

impl ConfigParam for Discrimination {
    const TAG: Tag = Tag::unchecked_new(2);
    const NAME: &'static str = "discrimination";

    fn to_payload(&self) -> TagPayload {
        match self {
            Discrimination::Production => vec![VAL_PROD],
            Discrimination::Test => vec![VAL_TEST],
        }
    }

    fn from_payload(payload: &TagPayload) -> Result<Self, Error> {
        if payload.len() != 1 {
            return Err(Error::SizeInvalid);
        };
        match payload[0] {
            VAL_PROD => Ok(Discrimination::Production),
            VAL_TEST => Ok(Discrimination::Test),
            _ => Err(Error::StructureInvalid),
        }
    }

    fn to_string(&self) -> String {
        match self {
            Discrimination::Production => "production".to_string(),
            Discrimination::Test => "test".to_string(),
        }
    }

    fn from_string(s: &str) -> Result<Self, Error> {
        match s {
            "production" => Ok(Discrimination::Production),
            "test" => Ok(Discrimination::Test),
            _ => Err(Error::UnknownString(s.to_string())),
        }
    }
}

impl ConfigParam for ConsensusVersion {
    const TAG: Tag = Tag::unchecked_new(3);
    const NAME: &'static str = "block0-consensus";

    fn to_payload(&self) -> TagPayload {
        (*self as u16).to_be_bytes().to_vec()
    }

    fn from_payload(payload: &TagPayload) -> Result<Self, Error> {
        let mut bytes = 0u16.to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(&payload);
        let integer = u16::from_be_bytes(bytes);
        ConsensusVersion::from_u16(integer).ok_or(Error::StructureInvalid)
    }

    fn to_string(&self) -> String {
        format!("{}", self)
    }

    fn from_string(s: &str) -> Result<Self, Error> {
        s.parse().map_err(|_| Error::UnknownString(s.to_string()))
    }
}

pub fn entity_to<T: ConfigParam>(t: &T) -> (Tag, TagPayload) {
    (T::TAG, t.to_payload())
}

pub fn entity_from<T: ConfigParam>(tag: Tag, payload: &TagPayload) -> Result<T, Error> {
    if tag != T::TAG {
        return Err(Error::InvalidTag);
    }
    T::from_payload(payload)
}

pub fn entity_to_string<T: ConfigParam>(t: &T) -> (&'static str, String) {
    (T::NAME, t.to_string())
}

pub fn entity_from_string<T: ConfigParam>(tag: &str, value: &str) -> Result<T, Error> {
    if tag != T::NAME {
        return Err(Error::InvalidTag);
    }
    T::from_string(value)
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::InvalidTag => write!(f, "Invalid tag"),
            Error::SizeInvalid => write!(f, "Invalid payload size"),
            Error::StructureInvalid => write!(f, "Invalid payload structure"),
            Error::UnknownString(s) => write!(f, "Invalid payload string: {}", s),
        }
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Block0Date {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block0Date(Arbitrary::arbitrary(g))
        }
    }
}
