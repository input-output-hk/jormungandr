use crate::stake::{StakeKeyId, StakePoolId, StakePoolInfo};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, Clone)]
pub struct SignatureRaw(Vec<u8>);

impl property::Serialize for SignatureRaw {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        writer.write_all(self.0.as_ref())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub content: CertificateContent,
    //pub signatures: Vec<SignatureRaw>,
}

#[derive(Debug, Clone)]
pub enum CertificateContent {
    StakeKeyRegistration(StakeKeyRegistration),
    StakeKeyDeregistration(StakeKeyDeregistration),
    StakeDelegation(StakeDelegation),
    StakePoolRegistration(StakePoolInfo),
    StakePoolRetirement(StakePoolRetirement),
}

#[derive(FromPrimitive)]
enum CertificateTag {
    StakeKeyRegistration = 1,
    StakeKeyDeregistration = 2,
    StakeDelegation = 3,
    StakePoolRegistration = 4,
    StakePoolRetirement = 5,
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        match &self.content {
            CertificateContent::StakeKeyRegistration(s) => {
                codec.put_u8(CertificateTag::StakeKeyRegistration as u8)?;
                s.serialize(&mut codec)
            }
            CertificateContent::StakeKeyDeregistration(s) => {
                codec.put_u8(CertificateTag::StakeKeyDeregistration as u8)?;
                s.serialize(&mut codec)
            }
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
        /* FIXME
        codec.put_u8(self.signatures.len() as u8)?;
        for sig in &self.signatures {
            sig.serialize(&mut codec)?;
        }
        */
        Ok(())
    }
}

impl Readable for Certificate {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        let tag = buf.get_u8()?;
        let content = match CertificateTag::from_u8(tag) {
            Some(CertificateTag::StakeKeyRegistration) => {
                CertificateContent::StakeKeyRegistration(StakeKeyRegistration::read(buf)?)
            }
            Some(CertificateTag::StakeKeyDeregistration) => {
                CertificateContent::StakeKeyDeregistration(StakeKeyDeregistration::read(buf)?)
            }
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
pub struct StakeKeyRegistration {
    pub stake_key_id: StakeKeyId,
}

/*
impl StakeKeyRegistration {
    pub fn make_certificate(self, stake_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakeKeyRegistration(Signed {
            sig: make_signature(stake_private_key, &self),
            data: self,
        })
    }
}
*/

impl property::Serialize for StakeKeyRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_key_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeKeyRegistration {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakeKeyRegistration {
            stake_key_id: StakeKeyId::read(buf)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyDeregistration {
    pub stake_key_id: StakeKeyId,
}

/*
impl StakeKeyDeregistration {
    pub fn make_certificate(self, stake_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakeKeyDeregistration(Signed {
            sig: make_signature(stake_private_key, &self),
            data: self,
        })
    }
}
*/

impl property::Serialize for StakeKeyDeregistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_key_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeKeyDeregistration {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakeKeyDeregistration {
            stake_key_id: StakeKeyId::read(buf)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub stake_key_id: StakeKeyId,
    pub pool_id: StakePoolId,
}

/*
impl StakeDelegation {
    pub fn make_certificate(self, stake_private_key: &SecretKey<Ed25519Extended>) -> Message {
        // FIXME: "It must be signed by sks_source, and that key must
        // be included in the witness." - why?
        Message::StakeDelegation(Signed {
            sig: make_signature(stake_private_key, &self),
            data: self,
        })
    }
}
*/

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_key_id.serialize(&mut codec)?;
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakeDelegation {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakeDelegation {
            stake_key_id: StakeKeyId::read(buf)?,
            pool_id: StakePoolId::read(buf)?,
        })
    }
}

/*
impl StakePoolInfo {
    /// Create a certificate for this stake pool registration, signed
    /// by the pool's staking key and the owners.
    pub fn make_certificate(self, pool_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakePoolRegistration(Signed {
            sig: make_signature(pool_private_key, &self),
            data: self,
        })
    }
}
*/

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_id: StakePoolId,
    // TODO: add epoch when the retirement will take effect
}

/*
impl StakePoolRetirement {
    /// Create a certificate for this stake pool retirement, signed
    /// by the pool's staking key.
    pub fn make_certificate(self, pool_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakePoolRetirement(Signed {
            sig: make_signature(pool_private_key, &self),
            data: self,
        })
    }
}
*/

impl property::Serialize for StakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for StakePoolRetirement {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(StakePoolRetirement {
            pool_id: StakePoolId::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::leadership::genesis::GenesisPraosLeader;
    use chain_crypto::SecretKey;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Certificate {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let content = match g.next_u32() % 5 {
                0 => CertificateContent::StakeKeyRegistration(Arbitrary::arbitrary(g)),
                1 => CertificateContent::StakeKeyDeregistration(Arbitrary::arbitrary(g)),
                2 => CertificateContent::StakeDelegation(Arbitrary::arbitrary(g)),
                3 => CertificateContent::StakePoolRegistration(Arbitrary::arbitrary(g)),
                _ => CertificateContent::StakePoolRetirement(Arbitrary::arbitrary(g)),
            };
            Certificate { content }
        }
    }

    impl Arbitrary for StakeKeyRegistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyRegistration {
                stake_key_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakeKeyDeregistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyDeregistration {
                stake_key_id: Arbitrary::arbitrary(g),
            }
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
            let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
            StakePoolInfo {
                serial: Arbitrary::arbitrary(g),
                owners: vec![Arbitrary::arbitrary(g)],
                initial_key: GenesisPraosLeader {
                    vrf_public_key: SecretKey::generate(&mut rng).to_public(),
                    kes_public_key: SecretKey::generate(&mut rng).to_public(),
                },
            }
        }
    }

    impl Arbitrary for StakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRetirement {
                pool_id: Arbitrary::arbitrary(g),
            }
        }
    }
}
