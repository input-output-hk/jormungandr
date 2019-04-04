use crate::block::ConsensusVersion;
use chain_addr::Discrimination;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::packer::Codec;
use chain_core::property;
use num_traits::FromPrimitive;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::InvalidTag => write!(f, "Invalid config parameter tag"),
            Error::SizeInvalid => write!(f, "Invalid config parameter size"),
            Error::StructureInvalid => write!(f, "Invalid config parameter structure"),
            Error::UnknownString(s) => write!(f, "Invalid config parameter string '{}'", s),
        }
    }
}

impl std::error::Error for Error {}

impl Into<ReadError> for Error {
    fn into(self) -> ReadError {
        ReadError::StructureInvalid(self.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigParam {
    Block0Date(Block0Date),
    Discrimination(Discrimination),
    ConsensusVersion(ConsensusVersion),
}

impl Readable for ConfigParam {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let taglen = TagLen(buf.get_u16()?);
        let bytes = buf.get_slice(taglen.get_len())?;
        match taglen.get_tag() {
            Block0Date::TAG => Block0Date::from_payload(bytes).map(ConfigParam::Block0Date),
            Discrimination::TAG => {
                Discrimination::from_payload(bytes).map(ConfigParam::Discrimination)
            }
            ConsensusVersion::TAG => {
                ConsensusVersion::from_payload(bytes).map(ConfigParam::ConsensusVersion)
            }
            _ => Err(Error::InvalidTag),
        }
        .map_err(Into::into)
    }
}

impl property::Serialize for ConfigParam {
    type Error = io::Error;

    fn serialize<W: Write>(&self, writer: W) -> Result<(), Self::Error> {
        let (tag, bytes) = match self {
            ConfigParam::Block0Date(data) => (Block0Date::TAG, data.to_payload()),
            ConfigParam::Discrimination(data) => (Discrimination::TAG, data.to_payload()),
            ConfigParam::ConsensusVersion(data) => (ConsensusVersion::TAG, data.to_payload()),
        };
        let taglen = TagLen::new(tag, bytes.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "initial ent payload too big".to_string(),
            )
        })?;
        let mut codec = Codec::from(writer);
        codec.put_u16(taglen.0)?;
        codec.write_all(&bytes)
    }
}

#[cfg(feature = "generic-serialization")]
mod serde_impl {
    use super::*;
    use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

    impl<'de> Deserialize<'de> for ConfigParam {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let (tag, value) = <(String, String)>::deserialize(deserializer)?;
            match &*tag {
                Block0Date::NAME => Block0Date::from_cfg_str(&value).map(ConfigParam::Block0Date),
                Discrimination::NAME => {
                    Discrimination::from_cfg_str(&value).map(ConfigParam::Discrimination)
                }
                ConsensusVersion::NAME => {
                    ConsensusVersion::from_cfg_str(&value).map(ConfigParam::ConsensusVersion)
                }
                _ => Err(Error::InvalidTag),
            }
            .map_err(D::Error::custom)
        }
    }

    impl Serialize for ConfigParam {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self {
                ConfigParam::Block0Date(data) => (Block0Date::NAME, data.to_cfg_string()),
                ConfigParam::Discrimination(data) => (Discrimination::NAME, data.to_cfg_string()),
                ConfigParam::ConsensusVersion(data) => {
                    (ConsensusVersion::NAME, data.to_cfg_string())
                }
            }
            .serialize(serializer)
        }
    }
}

trait ConfigParamVariant: Clone + Eq + PartialEq {
    const TAG: Tag;
    const NAME: &'static str;

    fn to_payload(&self) -> Vec<u8>;

    fn from_payload(payload: &[u8]) -> Result<Self, Error>;

    fn to_cfg_string(&self) -> String;
    fn from_cfg_str(s: &str) -> Result<Self, Error>;
}

impl ConfigParamVariant for Block0Date {
    const TAG: Tag = Tag::new(1);
    const NAME: &'static str = "block0-date";

    fn to_payload(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.0.to_be_bytes());
        out
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() != 8 {
            return Err(Error::SizeInvalid);
        };
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(payload);
        let v = u64::from_be_bytes(bytes);
        Ok(Block0Date(v))
    }

    fn to_cfg_string(&self) -> String {
        format!("{}", self.0).to_string()
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        let v = u64::from_str(s).map_err(|_| Error::UnknownString(s.to_string()))?;
        Ok(Block0Date(v))
    }
}

const VAL_PROD: u8 = 1;
const VAL_TEST: u8 = 2;

impl ConfigParamVariant for Discrimination {
    const TAG: Tag = Tag::new(2);
    const NAME: &'static str = "discrimination";

    fn to_payload(&self) -> Vec<u8> {
        match self {
            Discrimination::Production => vec![VAL_PROD],
            Discrimination::Test => vec![VAL_TEST],
        }
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() != 1 {
            return Err(Error::SizeInvalid);
        };
        match payload[0] {
            VAL_PROD => Ok(Discrimination::Production),
            VAL_TEST => Ok(Discrimination::Test),
            _ => Err(Error::StructureInvalid),
        }
    }

    fn to_cfg_string(&self) -> String {
        match self {
            Discrimination::Production => "production".to_string(),
            Discrimination::Test => "test".to_string(),
        }
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        match s {
            "production" => Ok(Discrimination::Production),
            "test" => Ok(Discrimination::Test),
            _ => Err(Error::UnknownString(s.to_string())),
        }
    }
}

impl ConfigParamVariant for ConsensusVersion {
    const TAG: Tag = Tag::new(3);
    const NAME: &'static str = "block0-consensus";

    fn to_payload(&self) -> Vec<u8> {
        (*self as u16).to_be_bytes().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut bytes = 0u16.to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(payload);
        let integer = u16::from_be_bytes(bytes);
        ConsensusVersion::from_u16(integer).ok_or(Error::StructureInvalid)
    }

    fn to_cfg_string(&self) -> String {
        format!("{}", self)
    }

    fn from_cfg_str(s: &str) -> Result<Self, Error> {
        s.parse().map_err(|_| Error::UnknownString(s.to_string()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Tag(u16);

impl Tag {
    pub const fn new(tag: u16) -> Self {
        // validate that it's less than 1024 when const fns support ifs
        Tag(tag)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TagLen(u16);

const MAXIMUM_LEN: usize = 64;

impl TagLen {
    pub fn new(tag: Tag, len: usize) -> Option<Self> {
        if len < MAXIMUM_LEN {
            Some(TagLen(tag.0 << 6 | len as u16))
        } else {
            None
        }
    }

    pub fn get_tag(self) -> Tag {
        Tag::new(self.0 >> 6)
    }

    pub fn get_len(self) -> usize {
        (self.0 & 0b11_1111) as usize
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};

    quickcheck! {
        fn tag_len_computation_correct(tag: Tag, len: usize) -> TestResult {
            let len = len % MAXIMUM_LEN;
            let tag_len = TagLen::new(tag, len).unwrap();

            assert_eq!(tag, tag_len.get_tag(), "Invalid tag");
            assert_eq!(len, tag_len.get_len(), "Invalid len");
            TestResult::passed()
        }
    }

    impl Arbitrary for Tag {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Tag::new(u16::arbitrary(g) % 1024)
        }
    }

    impl Arbitrary for Block0Date {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block0Date(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => ConfigParam::Block0Date(Arbitrary::arbitrary(g)),
                1 => ConfigParam::Discrimination(Arbitrary::arbitrary(g)),
                2 => ConfigParam::ConsensusVersion(Arbitrary::arbitrary(g)),
                _ => unreachable!(),
            }
        }
    }
}
