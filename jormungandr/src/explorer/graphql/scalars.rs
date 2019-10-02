use crate::blockcfg;
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::value;
use juniper;
use std::convert::TryFrom;

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
pub struct Serial(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct PublicKey(pub String);

#[derive(juniper::GraphQLScalarValue)]
pub struct TimeOffsetSeconds(pub String);

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

impl From<u32> for BlockCount {
    fn from(number: u32) -> BlockCount {
        BlockCount(format!("{}", number))
    }
}

impl From<u128> for Serial {
    fn from(number: u128) -> Serial {
        Serial(format!("{}", number))
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
