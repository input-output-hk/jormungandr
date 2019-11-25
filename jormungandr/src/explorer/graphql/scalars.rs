use super::error::ErrorKind;
use crate::blockcfg;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::value;
use juniper;
use juniper::{ParseScalarResult, ParseScalarValue};
use std::convert::{TryFrom, TryInto};

#[derive(juniper::GraphQLScalarValue)]
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
pub struct PublicKey(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct TimeOffsetSeconds(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct NonZero(pub String);

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

impl From<u32> for IndexCursor {
    fn from(number: u32) -> IndexCursor {
        IndexCursor(number.into())
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
        number.0.into()
    }
}

impl From<u64> for IndexCursor {
    fn from(number: u64) -> IndexCursor {
        IndexCursor(number.into())
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
