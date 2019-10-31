use crate::accounting::account::DelegationType;
use crate::certificate::{CertificateSlice, PoolId};
use crate::transaction::{
    AccountBindingSignature, AccountIdentifier, Payload, PayloadAuthData, PayloadData, PayloadSlice,
};

use chain_core::{
    mempack::{ReadBuf, ReadError, Readable},
    property,
};
use std::marker::PhantomData;
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
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }
    fn payload_auth_data(_: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(Vec::with_capacity(0).into(), PhantomData)
    }
    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        Some(CertificateSlice::from(p))
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
    type Auth = AccountBindingSignature;
    fn payload_data(&self) -> PayloadData<Self> {
        PayloadData(
            self.serialize_in(ByteBuilder::new())
                .finalize_as_vec()
                .into(),
            PhantomData,
        )
    }

    fn payload_auth_data(auth: &Self::Auth) -> PayloadAuthData<Self> {
        PayloadAuthData(auth.as_ref().to_owned().into(), PhantomData)
    }
    fn to_certificate_slice<'a>(p: PayloadSlice<'a, Self>) -> Option<CertificateSlice<'a>> {
        Some(CertificateSlice::from(p))
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
