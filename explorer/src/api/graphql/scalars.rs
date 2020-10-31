use super::error::Error;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{header, value, vote};
use juniper::{ParseScalarResult, ParseScalarValue};
use std::convert::{TryFrom, TryInto};

#[derive(Clone, juniper::GraphQLScalarValue)]
pub struct Slot(pub String);

#[derive(juniper::GraphQLScalarValue)]
/// Custom scalar type that represents a block's position in the blockchain.
/// It's a either 0 (the genesis block) or a positive number in string representation.
pub struct ChainLength(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct PoolId(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct Value(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct EpochNumber(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct BlockCount(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct TransactionCount(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct PoolCount(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct PublicKey(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct TimeOffsetSeconds(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct NonZero(pub String);

#[derive(Clone, juniper::GraphQLScalarValue)]
pub struct VotePlanId(pub String);

#[derive(Clone, juniper::GraphQLScalarValue)]
pub struct ExternalProposalId(pub String);

#[derive(Clone, juniper::GraphQLEnum)]
pub enum PayloadType {
    Public,
}

#[derive(Clone, juniper::GraphQLScalarValue)]
pub struct Weight(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct VotePlanCount(pub String);

/// Vote option range
///
/// provide a range of available choices for a given proposal. Usual value would
/// be `[0, 3[` (or `0..3` in rust's range syntax), meaning there are 3 options
/// available: `0`, `1` and `2`
#[derive(Clone, juniper::GraphQLObject)]
pub struct VoteOptionRange {
    /// the start of the vote option range, starting from 0 usually
    start: i32,
    /// the exclusive upper bound of the option range. minimal value is 1
    end: i32,
}

// u32 should be enough to count blocks and transactions (the only two cases for now)
#[derive(Clone)]
pub struct IndexCursor(pub u64);

juniper::graphql_scalar!(IndexCursor where Scalar = <S> {
    description: "Non-opaque cursor that can be used for offset-based pagination"

    resolve(&self) -> Value {
        juniper::Value::scalar(self.0.to_string())
    }

    from_input_value(v: &InputValue) -> Option<IndexCursor> {
        v.as_scalar_value::<String>()
         .and_then(|s| s.parse::<u64>().ok())
         .map(IndexCursor)
    }

    from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, S> {
        <String as ParseScalarValue<S>>::from_str(value)
    }
});

/*------------------------------*/
/*------- Conversions ---------*/
/*----------------------------*/

impl From<chain_impl_mockchain::header::ChainLength> for ChainLength {
    fn from(length: header::ChainLength) -> ChainLength {
        ChainLength(u32::from(length).to_string())
    }
}

impl TryFrom<ChainLength> for header::ChainLength {
    type Error = std::num::ParseIntError;
    fn try_from(
        length: ChainLength,
    ) -> Result<chain_impl_mockchain::header::ChainLength, Self::Error> {
        length.0.parse::<u32>().map(header::ChainLength::from)
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

impl From<header::Epoch> for EpochNumber {
    fn from(e: header::Epoch) -> EpochNumber {
        EpochNumber(format!("{}", e))
    }
}

impl TryFrom<EpochNumber> for header::Epoch {
    type Error = std::num::ParseIntError;
    fn try_from(e: EpochNumber) -> Result<header::Epoch, Self::Error> {
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
    type Error = Error;
    fn try_from(c: IndexCursor) -> Result<u32, Self::Error> {
        c.0.try_into().map_err(|_| {
            Error::InvalidCursor(
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

impl From<header::ChainLength> for IndexCursor {
    fn from(length: header::ChainLength) -> IndexCursor {
        IndexCursor(u32::from(length).into())
    }
}

impl TryFrom<IndexCursor> for header::ChainLength {
    type Error = Error;
    fn try_from(c: IndexCursor) -> Result<header::ChainLength, Self::Error> {
        let inner: u32 = c.0.try_into().map_err(|_| {
            Error::InvalidCursor(
                "block's pagination cursor is greater than maximum ChainLength".to_owned(),
            )
        })?;
        Ok(header::ChainLength::from(inner))
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
