use super::error::ErrorKind;
use crate::blockcfg;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{value, vote};
// use juniper::{ParseScalarResult, ParseScalarValue};
use async_graphql::*;
use std::convert::{TryFrom, TryInto};

#[derive(Clone)]
pub struct Slot(pub String);

#[Scalar]
impl ScalarType for Slot {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(Slot)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

/// Custom scalar type that represents a block's position in the blockchain.
/// It's a either 0 (the genesis block) or a positive number in string representation.
pub struct ChainLength(pub String);

#[Scalar]
impl ScalarType for ChainLength {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(ChainLength)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct PoolId(pub String);

#[Scalar]
impl ScalarType for PoolId {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(PoolId)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct Value(pub String);
#[Scalar]
impl ScalarType for Value {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(Value)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct EpochNumber(pub String);
#[Scalar]
impl ScalarType for EpochNumber {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(EpochNumber)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct BlockCount(pub String);
#[Scalar]
impl ScalarType for BlockCount {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(BlockCount)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct TransactionCount(pub String);
#[Scalar]
impl ScalarType for TransactionCount {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(TransactionCount)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct PoolCount(pub String);
#[Scalar]
impl ScalarType for PoolCount {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(PoolCount)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct PublicKey(pub String);
#[Scalar]
impl ScalarType for PublicKey {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(PublicKey)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct TimeOffsetSeconds(pub String);
#[Scalar]
impl ScalarType for TimeOffsetSeconds {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(TimeOffsetSeconds)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct NonZero(pub String);
#[Scalar]
impl ScalarType for NonZero {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(NonZero)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

#[derive(Clone)]
pub struct VotePlanId(pub String);
#[Scalar]
impl ScalarType for VotePlanId {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(VotePlanId)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

#[derive(Clone)]
pub struct ExternalProposalId(pub String);
#[Scalar]
impl ScalarType for ExternalProposalId {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(ExternalProposalId)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Enum)]
pub enum PayloadType {
    Public,
    Private,
}

#[derive(Clone)]
pub struct Weight(pub String);

#[Scalar]
impl ScalarType for Weight {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(Weight)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct VotePlanCount(pub String);
#[Scalar]
impl ScalarType for VotePlanCount {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(VotePlanCount)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

pub struct VoteStatusCount(pub String);
#[Scalar]
impl ScalarType for VoteStatusCount {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(VoteStatusCount)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

/// Vote option range
///
/// provide a range of available choices for a given proposal. Usual value would
/// be `[0, 3[` (or `0..3` in rust's range syntax), meaning there are 3 options
/// available: `0`, `1` and `2`
#[derive(Clone, SimpleObject)]
pub struct VoteOptionRange {
    /// the start of the vote option range, starting from 0 usually
    start: i32,
    /// the exclusive upper bound of the option range. minimal value is 1
    end: i32,
}

// u32 should be enough to count blocks and transactions (the only two cases for now)

#[derive(Clone)]
pub struct IndexCursor(pub u64);

impl async_graphql::connection::CursorType for IndexCursor {
    type Error = std::num::ParseIntError;

    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        s.parse::<u64>().map(IndexCursor)
    }

    fn encode_cursor(&self) -> String {
        self.0.to_string()
    }
}

#[Scalar]
impl ScalarType for IndexCursor {
    fn parse(value: async_graphql::Value) -> InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            Ok(value.parse().map(IndexCursor)?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        async_graphql::Value::String(self.0.to_string())
    }
}

/*------------------------------*/
/*------- Conversions ---------*/
/*----------------------------*/

impl From<blockcfg::ChainLength> for ChainLength {
    fn from(length: blockcfg::ChainLength) -> ChainLength {
        ChainLength(u32::from(length).to_string())
    }
}

impl TryFrom<ChainLength> for blockcfg::ChainLength {
    type Error = std::num::ParseIntError;
    fn try_from(length: ChainLength) -> Result<blockcfg::ChainLength, Self::Error> {
        length.0.parse::<u32>().map(blockcfg::ChainLength::from)
    }
}

impl From<&value::Value> for Value {
    fn from(v: &value::Value) -> Value {
        Value(format!("{}", v))
    }
}

impl From<value::Value> for Value {
    fn from(v: value::Value) -> Value {
        (&v).into()
    }
}

impl From<blockcfg::Epoch> for EpochNumber {
    fn from(e: blockcfg::Epoch) -> EpochNumber {
        EpochNumber(format!("{}", e))
    }
}

impl TryFrom<EpochNumber> for blockcfg::Epoch {
    type Error = std::num::ParseIntError;
    fn try_from(e: EpochNumber) -> Result<blockcfg::Epoch, Self::Error> {
        e.0.parse::<u32>()
    }
}

impl From<u64> for BlockCount {
    fn from(number: u64) -> BlockCount {
        BlockCount(format!("{}", number))
    }
}

impl From<u32> for BlockCount {
    fn from(number: u32) -> BlockCount {
        BlockCount(format!("{}", number))
    }
}

impl From<&chain_crypto::PublicKey<chain_crypto::Ed25519>> for PublicKey {
    fn from(pk: &chain_crypto::PublicKey<chain_crypto::Ed25519>) -> PublicKey {
        PublicKey(pk.to_bech32_str())
    }
}

impl From<chain_time::TimeOffsetSeconds> for TimeOffsetSeconds {
    fn from(time: chain_time::TimeOffsetSeconds) -> TimeOffsetSeconds {
        TimeOffsetSeconds(format!("{}", u64::from(time)))
    }
}

impl From<u64> for TransactionCount {
    fn from(n: u64) -> TransactionCount {
        TransactionCount(format!("{}", n))
    }
}

impl From<u64> for PoolCount {
    fn from(n: u64) -> PoolCount {
        PoolCount(format!("{}", n))
    }
}

impl From<u32> for IndexCursor {
    fn from(number: u32) -> IndexCursor {
        IndexCursor(number.into())
    }
}

impl From<chain_impl_mockchain::certificate::VotePlanId> for VotePlanId {
    fn from(id: chain_impl_mockchain::certificate::VotePlanId) -> VotePlanId {
        VotePlanId(id.to_string())
    }
}

impl From<vote::PayloadType> for PayloadType {
    fn from(payload_type: vote::PayloadType) -> Self {
        match payload_type {
            vote::PayloadType::Public => Self::Public,
            vote::PayloadType::Private => Self::Private,
        }
    }
}

impl From<vote::Options> for VoteOptionRange {
    fn from(options: vote::Options) -> Self {
        let range = options.choice_range();
        Self {
            start: range.start as i32,
            end: range.end as i32,
        }
    }
}

impl TryFrom<IndexCursor> for u32 {
    type Error = ErrorKind;
    fn try_from(c: IndexCursor) -> Result<u32, Self::Error> {
        c.0.try_into().map_err(|_| {
            ErrorKind::InvalidCursor(
                "block's pagination cursor is greater than maximum 2^32".to_owned(),
            )
        })
    }
}

impl From<IndexCursor> for u64 {
    fn from(number: IndexCursor) -> u64 {
        number.0
    }
}

impl From<u64> for IndexCursor {
    fn from(number: u64) -> IndexCursor {
        IndexCursor(number)
    }
}

impl From<blockcfg::ChainLength> for IndexCursor {
    fn from(length: blockcfg::ChainLength) -> IndexCursor {
        IndexCursor(u32::from(length).into())
    }
}

impl TryFrom<IndexCursor> for blockcfg::ChainLength {
    type Error = ErrorKind;
    fn try_from(c: IndexCursor) -> Result<blockcfg::ChainLength, Self::Error> {
        let inner: u32 = c.0.try_into().map_err(|_| {
            ErrorKind::InvalidCursor(
                "block's pagination cursor is greater than maximum ChainLength".to_owned(),
            )
        })?;
        Ok(blockcfg::ChainLength::from(inner))
    }
}

impl From<chain_impl_mockchain::certificate::ExternalProposalId> for ExternalProposalId {
    fn from(id: chain_impl_mockchain::certificate::ExternalProposalId) -> Self {
        ExternalProposalId(id.to_string())
    }
}

impl From<vote::Weight> for Weight {
    fn from(w: vote::Weight) -> Self {
        Self(format!("{}", w))
    }
}

impl From<u64> for VotePlanCount {
    fn from(number: u64) -> VotePlanCount {
        VotePlanCount(format!("{}", number))
    }
}

impl From<u32> for VotePlanCount {
    fn from(number: u32) -> VotePlanCount {
        VotePlanCount(format!("{}", number))
    }
}

impl From<u64> for VoteStatusCount {
    fn from(number: u64) -> VoteStatusCount {
        VoteStatusCount(format!("{}", number))
    }
}
