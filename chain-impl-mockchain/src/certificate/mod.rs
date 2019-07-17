use crate::stake::{StakePoolId, StakePoolInfo};
use crate::transaction::AccountIdentifier;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::Verification;

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
        match &self.content {
            CertificateContent::StakeDelegation(s) => {
                codec.put_u8(CertificateTag::StakeDelegation as u8)?;
                s.serialize(&mut codec)
            }
            CertificateContent::StakePoolRegistration(s) => {
                codec.put_u8(CertificateTag::StakePoolRegistration as u8)?;
                s.serialize(&mut codec)
            }
            CertificateContent::StakePoolRetirement(s) => {
                codec.put_u8(CertificateTag::StakePoolRetirement as u8)?;
                s.serialize(&mut codec)
            }
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

            None => panic!("not a certificate"),
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::leadership::genesis::GenesisPraosLeader;
    use chain_crypto::{Curve25519_2HashDH, PublicKey, SecretKey, SumEd25519_12};
    use lazy_static::lazy_static;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Certificate {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let content = match g.next_u32() % 3 {
                0 => CertificateContent::StakeDelegation(Arbitrary::arbitrary(g)),
                1 => CertificateContent::StakePoolRegistration(Arbitrary::arbitrary(g)),
                _ => CertificateContent::StakePoolRetirement(Arbitrary::arbitrary(g)),
            };
            Certificate { content }
        }
    }
    impl Arbitrary for StakeDelegation {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeDelegation {
                stake_key_id: Arbitrary::arbitrary(g),
                pool_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakePoolInfo {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            lazy_static! {
                static ref PK_KES: PublicKey<SumEd25519_12> = {
                    let sk: SecretKey<SumEd25519_12> =
                        SecretKey::generate(&mut rand_chacha::ChaChaRng::from_seed([0; 32]));
                    sk.to_public()
                };
            }
            let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
            let vrf_sk: SecretKey<Curve25519_2HashDH> = SecretKey::generate(&mut rng);
            StakePoolInfo {
                serial: Arbitrary::arbitrary(g),
                owners: vec![Arbitrary::arbitrary(g)],
                initial_key: GenesisPraosLeader {
                    vrf_public_key: vrf_sk.to_public(),
                    kes_public_key: PK_KES.clone(),
                },
            }
        }
    }

    impl Arbitrary for StakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRetirement {
                pool_id: Arbitrary::arbitrary(g),
                pool_info: Arbitrary::arbitrary(g),
            }
        }
    }
}
