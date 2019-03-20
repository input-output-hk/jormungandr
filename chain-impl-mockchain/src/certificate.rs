use crate::{
    block::Message,
    key::*,
    stake::{StakeKeyId, StakePoolId, StakePoolInfo},
};
use chain_core::property;
use chain_crypto::{Ed25519Extended, SecretKey};

#[derive(Debug, Clone)]
pub struct SignatureRaw(Vec<u8>);

impl property::Serialize for SignatureRaw {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        writer.write_all(self.0.as_ref())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Certificate {
    pub content: CertificateContent,
    pub signatures: Vec<SignatureRaw>,
}

#[derive(Debug, Clone)]
pub enum CertificateContent {
    StakeKeyRegistration(StakeKeyRegistration),
    StakeKeyDeregistration(StakeKeyDeregistration),
    StakeDelegation(StakeDelegation),
    StakePoolRegistration(StakePoolRegistration),
    StakePoolRetirement(StakePoolRetirement),
}

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        let tag = match &self.content {
            CertificateContent::StakeKeyRegistration(_) => 0x01,
            CertificateContent::StakeKeyDeregistration(_) => 0x02,
            CertificateContent::StakeDelegation(_) => 0x03,
            CertificateContent::StakePoolRegistration(_) => 0x04,
            CertificateContent::StakePoolRetirement(_) => 0x05,
        };
        codec.put_u8(tag)?;
        match &self.content {
            CertificateContent::StakeKeyRegistration(s) => s.serialize(&mut codec),
            CertificateContent::StakeKeyDeregistration(s) => s.serialize(&mut codec),
            CertificateContent::StakeDelegation(s) => s.serialize(&mut codec),
            CertificateContent::StakePoolRegistration(s) => s.serialize(&mut codec),
            CertificateContent::StakePoolRetirement(s) => s.serialize(&mut codec),
        }?;
        codec.put_u8(self.signatures.len() as u8)?;
        for sig in &self.signatures {
            sig.serialize(&mut codec)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyRegistration {
    pub stake_key_id: StakeKeyId,
}

impl StakeKeyRegistration {
    pub fn make_certificate(self, stake_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakeKeyRegistration(Signed {
            sig: make_signature(stake_private_key, &self),
            data: self,
        })
    }
}

impl property::Serialize for StakeKeyRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_key_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakeKeyRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakeKeyRegistration {
            stake_key_id: StakeKeyId::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyDeregistration {
    pub stake_key_id: StakeKeyId,
}

impl StakeKeyDeregistration {
    pub fn make_certificate(self, stake_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakeKeyDeregistration(Signed {
            sig: make_signature(stake_private_key, &self),
            data: self,
        })
    }
}

impl property::Serialize for StakeKeyDeregistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_key_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakeKeyDeregistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakeKeyDeregistration {
            stake_key_id: StakeKeyId::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub stake_key_id: StakeKeyId,
    pub pool_id: StakePoolId,
}

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

impl property::Deserialize for StakeDelegation {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakeDelegation {
            stake_key_id: StakeKeyId::deserialize(&mut codec)?,
            pool_id: StakePoolId::deserialize(&mut codec)?,
        })
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_id: StakePoolId,
    // TODO: add epoch when the retirement will take effect
}

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

impl property::Serialize for StakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.pool_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakePoolRetirement {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakePoolRetirement {
            pool_id: StakePoolId::deserialize(&mut codec)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::leadership::genesis::GenesisPraosLeader;
    use quickcheck::{Arbitrary, Gen};

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
