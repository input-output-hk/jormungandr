use crate::leadership::bft::LeaderId;
use crate::milli::Milli;
use crate::value::Value;
use crate::rewards::{Ratio, TaxType};
use crate::{block::ConsensusVersion, fee::LinearFee};
use chain_addr::Discrimination;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::packer::Codec;
use chain_core::property;
use chain_crypto::PublicKey;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Write};
use strum_macros::{AsRefStr, EnumIter, EnumString};
use typed_bytes::ByteBuilder;

/// Possible errors
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    InvalidTag,
    SizeInvalid,
    BoolInvalid,
    StructureInvalid,
    UnknownString(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Error::InvalidTag => write!(f, "Invalid config parameter tag"),
            Error::SizeInvalid => write!(f, "Invalid config parameter size"),
            Error::BoolInvalid => write!(f, "Invalid Boolean in config parameter"),
            Error::StructureInvalid => write!(f, "Invalid config parameter structure"),
            Error::UnknownString(s) => write!(f, "Invalid config parameter string '{}'", s),
        }
    }
}

impl std::error::Error for Error {}

impl From<ReadError> for Error {
    fn from(_: ReadError) -> Self {
        Error::StructureInvalid
    }
}

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
    SlotsPerEpoch(u32),
    SlotDuration(u8),
    EpochStabilityDepth(u32),
    ConsensusGenesisPraosActiveSlotsCoeff(Milli),
    MaxNumberOfTransactionsPerBlock(u32),
    BftSlotsRatio(Milli),
    AddBftLeader(LeaderId),
    RemoveBftLeader(LeaderId),
    LinearFee(LinearFee),
    ProposalExpiration(u32),
    KESUpdateSpeed(u32),
    TreasuryAdd(Value),
    TreasuryParams(TaxType),
    RewardPot(Value),
    RewardParams(RewardParams),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardParams {
    Linear(u64, u64, u64),
    Halving(u64, u64, u64),
}

// Discriminants can NEVER be 1024 or higher
#[derive(AsRefStr, Clone, Copy, Debug, EnumIter, EnumString, PartialEq)]
pub enum Tag {
    #[strum(to_string = "discrimination")]
    Discrimination = 1,
    #[strum(to_string = "block0-date")]
    Block0Date = 2,
    #[strum(to_string = "block0-consensus")]
    ConsensusVersion = 3,
    #[strum(to_string = "slots-per-epoch")]
    SlotsPerEpoch = 4,
    #[strum(to_string = "slot-duration")]
    SlotDuration = 5,
    #[strum(to_string = "epoch-stability-depth")]
    EpochStabilityDepth = 6,
    #[strum(to_string = "genesis-praos-param-f")]
    ConsensusGenesisPraosActiveSlotsCoeff = 8,
    #[strum(to_string = "max-number-of-transactions-per-block")]
    MaxNumberOfTransactionsPerBlock = 9,
    #[strum(to_string = "bft-slots-ratio")]
    BftSlotsRatio = 10,
    #[strum(to_string = "add-bft-leader")]
    AddBftLeader = 11,
    #[strum(to_string = "remove-bft-leader")]
    RemoveBftLeader = 12,
    #[strum(to_string = "linear-fee")]
    LinearFee = 14,
    #[strum(to_string = "proposal-expiration")]
    ProposalExpiration = 15,
    #[strum(to_string = "kes-update-speed")]
    KESUpdateSpeed = 16,
    #[strum(to_string = "treasury")]
    TreasuryAdd = 17,
    #[strum(to_string = "treasury-params")]
    TreasuryParams = 18,
    #[strum(to_string = "reward-pot")]
    RewardPot = 19,
    #[strum(to_string = "reward-params")]
    RewardParams = 20,
}

impl Tag {
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Tag::Discrimination),
            2 => Some(Tag::Block0Date),
            3 => Some(Tag::ConsensusVersion),
            4 => Some(Tag::SlotsPerEpoch),
            5 => Some(Tag::SlotDuration),
            6 => Some(Tag::EpochStabilityDepth),
            8 => Some(Tag::ConsensusGenesisPraosActiveSlotsCoeff),
            9 => Some(Tag::MaxNumberOfTransactionsPerBlock),
            10 => Some(Tag::BftSlotsRatio),
            11 => Some(Tag::AddBftLeader),
            12 => Some(Tag::RemoveBftLeader),
            14 => Some(Tag::LinearFee),
            15 => Some(Tag::ProposalExpiration),
            16 => Some(Tag::KESUpdateSpeed),
            17 => Some(Tag::TreasuryAdd),
            18 => Some(Tag::TreasuryParams),
            19 => Some(Tag::RewardPot),
            20 => Some(Tag::RewardParams),
            _ => None,
        }
    }
}

impl<'a> From<&'a ConfigParam> for Tag {
    fn from(config_param: &'a ConfigParam) -> Self {
        match config_param {
            ConfigParam::Block0Date(_) => Tag::Block0Date,
            ConfigParam::Discrimination(_) => Tag::Discrimination,
            ConfigParam::ConsensusVersion(_) => Tag::ConsensusVersion,
            ConfigParam::SlotsPerEpoch(_) => Tag::SlotsPerEpoch,
            ConfigParam::SlotDuration(_) => Tag::SlotDuration,
            ConfigParam::EpochStabilityDepth(_) => Tag::EpochStabilityDepth,
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(_) => {
                Tag::ConsensusGenesisPraosActiveSlotsCoeff
            }
            ConfigParam::MaxNumberOfTransactionsPerBlock(_) => Tag::MaxNumberOfTransactionsPerBlock,
            ConfigParam::BftSlotsRatio(_) => Tag::BftSlotsRatio,
            ConfigParam::AddBftLeader(_) => Tag::AddBftLeader,
            ConfigParam::RemoveBftLeader(_) => Tag::RemoveBftLeader,
            ConfigParam::LinearFee(_) => Tag::LinearFee,
            ConfigParam::ProposalExpiration(_) => Tag::ProposalExpiration,
            ConfigParam::KESUpdateSpeed(_) => Tag::KESUpdateSpeed,
            ConfigParam::TreasuryAdd(_) => Tag::TreasuryAdd,
            ConfigParam::TreasuryParams(_) => Tag::TreasuryParams,
            ConfigParam::RewardPot(_) => Tag::RewardPot,
            ConfigParam::RewardParams(_) => Tag::RewardParams,
        }
    }
}

impl Readable for ConfigParam {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let taglen = TagLen(buf.get_u16()?);
        let bytes = buf.get_slice(taglen.get_len())?;
        match taglen.get_tag().map_err(Into::into)? {
            Tag::Block0Date => ConfigParamVariant::from_payload(bytes).map(ConfigParam::Block0Date),
            Tag::Discrimination => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::Discrimination)
            }
            Tag::ConsensusVersion => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::ConsensusVersion)
            }
            Tag::SlotsPerEpoch => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::SlotsPerEpoch)
            }
            Tag::SlotDuration => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::SlotDuration)
            }
            Tag::EpochStabilityDepth => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::EpochStabilityDepth)
            }
            Tag::ConsensusGenesisPraosActiveSlotsCoeff => ConfigParamVariant::from_payload(bytes)
                .map(ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff),
            Tag::MaxNumberOfTransactionsPerBlock => ConfigParamVariant::from_payload(bytes)
                .map(ConfigParam::MaxNumberOfTransactionsPerBlock),
            Tag::BftSlotsRatio => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::BftSlotsRatio)
            }
            Tag::AddBftLeader => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::AddBftLeader)
            }
            Tag::RemoveBftLeader => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::RemoveBftLeader)
            }
            Tag::LinearFee => ConfigParamVariant::from_payload(bytes).map(ConfigParam::LinearFee),
            Tag::ProposalExpiration => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::ProposalExpiration)
            }
            Tag::KESUpdateSpeed => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::KESUpdateSpeed)
            }
            Tag::TreasuryAdd => ConfigParamVariant::from_payload(bytes).map(ConfigParam::TreasuryAdd),
            Tag::TreasuryParams => ConfigParamVariant::from_payload(bytes).map(ConfigParam::TreasuryParams),
            Tag::RewardPot => ConfigParamVariant::from_payload(bytes).map(ConfigParam::RewardPot),
            Tag::RewardParams => {
                ConfigParamVariant::from_payload(bytes).map(ConfigParam::RewardParams)
            }
        }
        .map_err(Into::into)
    }
}

impl property::Serialize for ConfigParam {
    type Error = io::Error;

    fn serialize<W: Write>(&self, writer: W) -> Result<(), Self::Error> {
        let tag = Tag::from(self);
        let bytes = match self {
            ConfigParam::Block0Date(data) => data.to_payload(),
            ConfigParam::Discrimination(data) => data.to_payload(),
            ConfigParam::ConsensusVersion(data) => data.to_payload(),
            ConfigParam::SlotsPerEpoch(data) => data.to_payload(),
            ConfigParam::SlotDuration(data) => data.to_payload(),
            ConfigParam::EpochStabilityDepth(data) => data.to_payload(),
            ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(data) => data.to_payload(),
            ConfigParam::MaxNumberOfTransactionsPerBlock(data) => data.to_payload(),
            ConfigParam::BftSlotsRatio(data) => data.to_payload(),
            ConfigParam::AddBftLeader(data) => data.to_payload(),
            ConfigParam::RemoveBftLeader(data) => data.to_payload(),
            ConfigParam::LinearFee(data) => data.to_payload(),
            ConfigParam::ProposalExpiration(data) => data.to_payload(),
            ConfigParam::KESUpdateSpeed(data) => data.to_payload(),
            ConfigParam::TreasuryAdd(data) => data.to_payload(),
            ConfigParam::TreasuryParams(data) => data.to_payload(),
            ConfigParam::RewardPot(data) => data.to_payload(),
            ConfigParam::RewardParams(data) => data.to_payload(),
        };
        let taglen = TagLen::new(tag, bytes.len()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "initial ent payload too big".to_string(),
            )
        })?;
        let mut codec = Codec::new(writer);
        codec.put_u16(taglen.0)?;
        codec.write_all(&bytes)
    }
}

trait ConfigParamVariant: Clone + Eq + PartialEq {
    fn to_payload(&self) -> Vec<u8>;
    fn from_payload(payload: &[u8]) -> Result<Self, Error>;
}

/// Seconds elapsed since 1-Jan-1970 (unix time)
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Block0Date(pub u64);

impl ConfigParamVariant for Block0Date {
    fn to_payload(&self) -> Vec<u8> {
        self.0.to_payload()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        u64::from_payload(payload).map(Block0Date)
    }
}

impl ConfigParamVariant for TaxType {
    fn to_payload(&self) -> Vec<u8> {
        let bb: ByteBuilder<TaxType> =
            ByteBuilder::new().u64(self.fixed.0)
                .u64(self.ratio.numerator)
                .u64(self.ratio.denominator.get())
                .u64(self.max_limit.map_or(0, |v| v.get()));
        bb.finalize_as_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        use std::num::NonZeroU64;

        let mut rb = ReadBuf::from(payload);
        let value = rb.get_u64().map(Value)?;
        let num = rb.get_u64()?;
        let denom = rb.get_u64()?;
        let limit = rb.get_u64()?;
        let denominator = NonZeroU64::new(denom).map_or_else(|| Err(Error::StructureInvalid), Ok)?;
        rb.expect_end()?;
        Ok(TaxType {
            fixed: value,
            ratio: Ratio { numerator: num, denominator },
            max_limit: NonZeroU64::new(limit),
        })
    }
}

impl ConfigParamVariant for RewardParams {
    fn to_payload(&self) -> Vec<u8> {
        let bb: ByteBuilder<RewardParams> = match self {
            RewardParams::Linear(start, num, denom) => {
                ByteBuilder::new().u8(1).u64(*start).u64(*num).u64(*denom)
            }
            RewardParams::Halving(start, num, denom) => {
                ByteBuilder::new().u8(2).u64(*start).u64(*num).u64(*denom)
            }
        };
        bb.finalize_as_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut rb = ReadBuf::from(payload);
        match rb.get_u8()? {
            1 => {
                let start = rb.get_u64()?;
                let num = rb.get_u64()?;
                let denom = rb.get_u64()?;
                rb.expect_end()?;
                Ok(RewardParams::Linear(start, num, denom))
            }
            2 => {
                let start = rb.get_u64()?;
                let num = rb.get_u64()?;
                let denom = rb.get_u64()?;
                rb.expect_end()?;
                Ok(RewardParams::Halving(start, num, denom))
            }
            _ => Err(Error::InvalidTag),
        }
    }
}

impl ConfigParamVariant for Value {
    fn to_payload(&self) -> Vec<u8> {
        self.0.to_payload()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        u64::from_payload(payload).map(Value)
    }
}

const VAL_PROD: u8 = 1;
const VAL_TEST: u8 = 2;

impl ConfigParamVariant for Discrimination {
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
}

impl ConfigParamVariant for ConsensusVersion {
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
}

impl ConfigParamVariant for LeaderId {
    fn to_payload(&self) -> Vec<u8> {
        self.as_ref().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        PublicKey::from_binary(payload)
            .map(Into::into)
            .map_err(|_| Error::SizeInvalid)
    }
}

impl ConfigParamVariant for bool {
    fn to_payload(&self) -> Vec<u8> {
        vec![if *self { 1 } else { 0 }]
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        match payload.len() {
            1 => match payload[0] {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::BoolInvalid),
            },
            _ => Err(Error::SizeInvalid),
        }
    }
}

impl ConfigParamVariant for u8 {
    fn to_payload(&self) -> Vec<u8> {
        vec![*self]
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        match payload.len() {
            1 => Ok(payload[0]),
            _ => Err(Error::SizeInvalid),
        }
    }
}

impl ConfigParamVariant for u64 {
    fn to_payload(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut bytes = Self::default().to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(payload);
        Ok(Self::from_be_bytes(bytes))
    }
}

impl ConfigParamVariant for u32 {
    fn to_payload(&self) -> Vec<u8> {
        self.to_be_bytes().to_vec()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        let mut bytes = Self::default().to_ne_bytes();
        if payload.len() != bytes.len() {
            return Err(Error::SizeInvalid);
        };
        bytes.copy_from_slice(payload);
        Ok(Self::from_be_bytes(bytes))
    }
}

impl ConfigParamVariant for Milli {
    fn to_payload(&self) -> Vec<u8> {
        self.to_millis().to_payload()
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        u64::from_payload(payload).map(Milli::from_millis)
    }
}

impl ConfigParamVariant for LinearFee {
    fn to_payload(&self) -> Vec<u8> {
        let mut v = self.constant.to_payload();
        v.extend(self.coefficient.to_payload());
        v.extend(self.certificate.to_payload());
        v
    }

    fn from_payload(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() != 3 * 8 {
            return Err(Error::SizeInvalid);
        }
        Ok(LinearFee {
            constant: u64::from_payload(&payload[0..8])?,
            coefficient: u64::from_payload(&payload[8..16])?,
            certificate: u64::from_payload(&payload[16..24])?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TagLen(u16);

const MAXIMUM_LEN: usize = 64;

impl TagLen {
    pub fn new(tag: Tag, len: usize) -> Option<Self> {
        if len < MAXIMUM_LEN {
            Some(TagLen((tag as u16) << 6 | len as u16))
        } else {
            None
        }
    }

    pub fn get_len(self) -> usize {
        (self.0 & 0b11_1111) as usize
    }

    pub fn get_tag(self) -> Result<Tag, Error> {
        Tag::from_u16(self.0 >> 6).ok_or(Error::InvalidTag)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use strum::IntoEnumIterator;

    quickcheck! {
        fn tag_len_computation_correct(tag: Tag, len: usize) -> TestResult {
            let len = len % MAXIMUM_LEN;
            let tag_len = TagLen::new(tag, len).unwrap();

            assert_eq!(Ok(tag), tag_len.get_tag(), "Invalid tag");
            assert_eq!(len, tag_len.get_len(), "Invalid len");
            TestResult::passed()
        }
    }

    impl Arbitrary for Tag {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let idx = usize::arbitrary(g) % Tag::iter().count();
            Tag::iter().nth(idx).unwrap()
        }
    }

    impl Arbitrary for Block0Date {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Block0Date(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for RewardParams {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match bool::arbitrary(g) {
                false => RewardParams::Linear(
                    Arbitrary::arbitrary(g),
                    Arbitrary::arbitrary(g),
                    Arbitrary::arbitrary(g),
                ),
                true => RewardParams::Halving(
                    Arbitrary::arbitrary(g),
                    Arbitrary::arbitrary(g),
                    Arbitrary::arbitrary(g),
                ),
            }
        }
    }

    impl Arbitrary for ConfigParam {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            match u8::arbitrary(g) % 15 {
                0 => ConfigParam::Block0Date(Arbitrary::arbitrary(g)),
                1 => ConfigParam::Discrimination(Arbitrary::arbitrary(g)),
                2 => ConfigParam::ConsensusVersion(Arbitrary::arbitrary(g)),
                3 => ConfigParam::SlotsPerEpoch(Arbitrary::arbitrary(g)),
                4 => ConfigParam::SlotDuration(Arbitrary::arbitrary(g)),
                5 => ConfigParam::ConsensusGenesisPraosActiveSlotsCoeff(Arbitrary::arbitrary(g)),
                6 => ConfigParam::MaxNumberOfTransactionsPerBlock(Arbitrary::arbitrary(g)),
                7 => ConfigParam::BftSlotsRatio(Arbitrary::arbitrary(g)),
                8 => ConfigParam::AddBftLeader(Arbitrary::arbitrary(g)),
                9 => ConfigParam::RemoveBftLeader(Arbitrary::arbitrary(g)),
                10 => ConfigParam::LinearFee(Arbitrary::arbitrary(g)),
                11 => ConfigParam::ProposalExpiration(Arbitrary::arbitrary(g)),
                12 => ConfigParam::TreasuryAdd(Arbitrary::arbitrary(g)),
                13 => ConfigParam::RewardPot(Arbitrary::arbitrary(g)),
                14 => ConfigParam::RewardParams(Arbitrary::arbitrary(g)),
                _ => unreachable!(),
            }
        }
    }
}
