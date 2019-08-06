use crate::stake::StakePoolId;
use crate::transaction::AccountIdentifier;

use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
use typed_bytes::ByteBuilder;

/// A self delegation to a specific StakePoolId.
///
/// This structure is not sufficient to identify the owner, and instead we rely on a special
/// authenticated transaction, which has 1 input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStakeDelegation {
    pub stake_pool: StakePoolId,
}

impl OwnerStakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.stake_pool.as_ref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub account_id: AccountIdentifier,
    pub pool_id: StakePoolId,
}

impl StakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.account_id.as_ref())
            .bytes(self.pool_id.as_ref())
    }
}

impl property::Serialize for OwnerStakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.stake_pool.serialize(writer)
    }
}

impl Readable for OwnerStakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let stake_pool = StakePoolId::read(buf)?;
        Ok(Self { stake_pool })
    }
}

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;
        let mut codec = Codec::new(writer);
        codec.write_all(self.account_id.as_ref())?;
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let account_identifier = <[u8; 32]>::read(buf)?;
        Ok(StakeDelegation {
            account_id: account_identifier.into(),
            pool_id: StakePoolId::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for OwnerStakeDelegation {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                stake_pool: Arbitrary::arbitrary(g),
            }
        }
    }
}
