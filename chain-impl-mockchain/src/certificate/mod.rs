mod delegation;

#[cfg(test)]
mod test;

use crate::stake::{StakePoolId, StakePoolInfo};
use crate::transaction::AccountIdentifier;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::Verification;

pub use delegation::OwnerStakeDelegation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Certificate {
    pub content: CertificateContent,
}

impl Certificate {
    pub fn verify(&self) -> Verification {
        Verification::Success
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CertificateContent {
    StakeDelegation(StakeDelegation),
    StakePoolRegistration(StakePoolInfo),
    StakePoolRetirement(StakePoolRetirement),
}

impl CertificateContent {
    fn get_certificate_tag(&self) -> CertificateTag {
        match self {
            CertificateContent::StakeDelegation(_) => CertificateTag::StakeDelegation,
            CertificateContent::StakePoolRegistration(_) => CertificateTag::StakePoolRegistration,
            CertificateContent::StakePoolRetirement(_) => CertificateTag::StakePoolRetirement,
        }
    }
}

enum CertificateTag {
    StakeDelegation = 1,
    StakePoolRegistration = 2,
    StakePoolRetirement = 3,
}

impl CertificateTag {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(CertificateTag::StakeDelegation),
            2 => Some(CertificateTag::StakePoolRegistration),
            3 => Some(CertificateTag::StakePoolRetirement),
            _ => None,
        }
    }
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        codec.put_u8(self.content.get_certificate_tag() as u8)?;
        match &self.content {
            CertificateContent::StakeDelegation(s) => s.serialize(&mut codec),
            CertificateContent::StakePoolRegistration(s) => s.serialize(&mut codec),
            CertificateContent::StakePoolRetirement(s) => s.serialize(&mut codec),
        }?;
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let tag = buf.get_u8()?;
        let content = match CertificateTag::from_u8(tag) {
            Some(CertificateTag::StakePoolRegistration) => {
                CertificateContent::StakePoolRegistration(StakePoolInfo::read(buf)?)
            }
            Some(CertificateTag::StakePoolRetirement) => {
                CertificateContent::StakePoolRetirement(StakePoolRetirement::read(buf)?)
            }
            Some(CertificateTag::StakeDelegation) => {
                CertificateContent::StakeDelegation(StakeDelegation::read(buf)?)
            }
            None => {
                return Err(ReadError::StructureInvalid(
                    "certificate unknown".to_string(),
                ))
            }
        };
        Ok(Certificate { content })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub stake_key_id: AccountIdentifier,
    pub pool_id: StakePoolId,
}

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        use std::io::Write;
        let mut codec = Codec::new(writer);
        codec.write_all(self.stake_key_id.as_ref())?;
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let account_identifier = <[u8; 32]>::read(buf)?;
        Ok(StakeDelegation {
            stake_key_id: account_identifier.into(),
            pool_id: StakePoolId::read(buf)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_id: StakePoolId,
    // TODO: add epoch when the retirement will take effect
    pub pool_info: StakePoolInfo,
}

impl property::Serialize for StakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.pool_id.serialize(&mut codec)?;
        self.pool_info.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakePoolRetirement {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakePoolRetirement {
            pool_id: StakePoolId::read(buf)?,
            pool_info: StakePoolInfo::read(buf)?,
        })
    }
}
