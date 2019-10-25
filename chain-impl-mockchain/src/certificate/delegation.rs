use crate::accounting::account::DelegationType;
use crate::certificate::PoolId;
use crate::transaction::{AccountIdentifier, Payload, TransactionBindingSignature};

use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
use chain_crypto::Ed25519;
use typed_bytes::ByteBuilder;

/// A self delegation to a specific StakePoolId.
///
/// This structure is not sufficient to identify the owner, and instead we rely on a special
/// authenticated transaction, which has 1 input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerStakeDelegation {
    pub pool_id: PoolId,
}

impl OwnerStakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.pool_id.as_ref())
    }

    pub fn get_delegation_type(&self) -> DelegationType {
        DelegationType::Full(self.pool_id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub account_id: AccountIdentifier,
    pub pool_id: PoolId,
}

impl StakeDelegation {
    pub fn serialize_in(&self, bb: ByteBuilder<Self>) -> ByteBuilder<Self> {
        bb.bytes(self.account_id.as_ref())
            .bytes(self.pool_id.as_ref())
    }

    pub fn get_delegation_type(&self) -> DelegationType {
        DelegationType::Full(self.pool_id.clone())
    }
}

impl property::Serialize for OwnerStakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.pool_id.as_ref())
    }
}

impl Readable for OwnerStakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let pool_id = <[u8; 32]>::read(buf)?.into();
        Ok(Self { pool_id })
    }
}

impl Payload for OwnerStakeDelegation {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = false;
    type Auth = ();
    fn to_bytes(&self) -> Vec<u8> {
        self.serialize_in(ByteBuilder::new()).finalize_as_vec()
    }
    fn auth_to_bytes(_: &Self::Auth) -> Vec<u8> {
        Vec::with_capacity(0)
    }
}

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;
        let mut codec = Codec::new(writer);
        codec.write_all(self.account_id.as_ref())?;
        codec.write_all(self.pool_id.as_ref())?;
        Ok(())
    }
}

impl Readable for StakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let account_identifier = <[u8; 32]>::read(buf)?;
        let pool_id = <[u8; 32]>::read(buf)?.into();
        Ok(StakeDelegation {
            account_id: account_identifier.into(),
            pool_id,
        })
    }
}

impl Payload for StakeDelegation {
    const HAS_DATA: bool = true;
    const HAS_AUTH: bool = true;
    type Auth = TransactionBindingSignature<Ed25519>;
    fn to_bytes(&self) -> Vec<u8> {
        self.serialize_in(ByteBuilder::new()).finalize_as_vec()
    }

    fn auth_to_bytes(auth: &Self::Auth) -> Vec<u8> {
        auth.as_ref().to_owned()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for OwnerStakeDelegation {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                pool_id: Arbitrary::arbitrary(g),
            }
        }
    }
}
