use crate::{block::Message, key::*, leadership::genesis::GenesisPraosId, stake::StakeKeyId};
use chain_core::property;
use chain_crypto::{Curve25519_2HashDH, Ed25519Extended, FakeMMM, PublicKey, SecretKey};

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
    pub pool_id: GenesisPraosId,
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
            pool_id: GenesisPraosId::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRegistration {
    pub pool_id: GenesisPraosId,
    pub owner: StakeKeyId, // FIXME: support list of owners
    // reward sharing params: cost, margin, pledged amount of stake
    // alternative stake key reward account
    pub kes_public_key: PublicKey<FakeMMM>,
    pub vrf_public_key: PublicKey<Curve25519_2HashDH>,
}

impl StakePoolRegistration {
    /// Create a certificate for this stake pool registration, signed
    /// by the pool's staking key and the owners.
    pub fn make_certificate(self, pool_private_key: &SecretKey<Ed25519Extended>) -> Message {
        Message::StakePoolRegistration(Signed {
            sig: make_signature(pool_private_key, &self),
            data: self,
        })
    }
}

impl property::Serialize for StakePoolRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        self.pool_id.serialize(&mut writer)?;
        writer.write_all(self.vrf_public_key.as_ref())?;
        serialize_public_key(&self.kes_public_key, &mut writer)?;
        self.owner.serialize(&mut writer)?;
        Ok(())
    }
}

impl property::Deserialize for StakePoolRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(mut reader: R) -> Result<Self, Self::Error> {
        let pool_id = GenesisPraosId::deserialize(&mut reader)?;
        let vrf_public_key = deserialize_public_key(&mut reader)?;
        let kes_public_key = deserialize_public_key(&mut reader)?;
        let owner = StakeKeyId::deserialize(&mut reader)?;
        Ok(StakePoolRegistration {
            pool_id: pool_id,
            vrf_public_key: vrf_public_key,
            kes_public_key: kes_public_key,
            owner: owner,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_id: GenesisPraosId,
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
            pool_id: GenesisPraosId::deserialize(&mut codec)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
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

    impl Arbitrary for StakePoolRegistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = rand_chacha::ChaChaRng::from_seed(seed);
            StakePoolRegistration {
                pool_id: Arbitrary::arbitrary(g),
                vrf_public_key: SecretKey::generate(&mut rng).to_public(),
                kes_public_key: SecretKey::generate(&mut rng).to_public(),
                owner: Arbitrary::arbitrary(g),
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
