use crate::key::*;
use chain_core::property;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    StakeKeyRegistration(SignedStakeKeyRegistration),
    //StakeKeyDeregistration(...),
    StakePoolRegistration(SignedStakePoolRegistration),
    StakePoolRetirement(SignedStakePoolRetirement),
}

pub const TAG_STAKE_KEY_REGISTRATION: u8 = 1;
pub const TAG_STAKE_POOL_REGISTRATION: u8 = 3;
pub const TAG_STAKE_POOL_RETIREMENT: u8 = 5;

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        match self {
            Certificate::StakeKeyRegistration(signed_reg) => {
                codec.put_u8(TAG_STAKE_KEY_REGISTRATION)?;
                signed_reg.serialize(&mut codec)
            }
            Certificate::StakePoolRegistration(signed_reg) => {
                codec.put_u8(TAG_STAKE_POOL_REGISTRATION)?;
                signed_reg.serialize(&mut codec)
            }
            Certificate::StakePoolRetirement(signed_ret) => {
                codec.put_u8(TAG_STAKE_POOL_RETIREMENT)?;
                signed_ret.serialize(&mut codec)
            }
        }
    }
}

impl property::Deserialize for Certificate {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        match codec.get_u8()? {
            TAG_STAKE_POOL_REGISTRATION => {
                Ok(Certificate::StakePoolRegistration(SignedStakePoolRegistration::deserialize(&mut codec)?))
            }
            n => panic!("Unrecognized certificate tag {}.", n) // FIXME: return Error
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedStakeKeyRegistration {
    pub data: StakeKeyRegistration,
    pub sig: Signature,
}

impl property::Serialize for SignedStakeKeyRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.data.serialize(&mut codec)?;
        self.sig.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for SignedStakeKeyRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let data = StakeKeyRegistration::deserialize(&mut codec)?;
        let sig = Signature::deserialize(&mut codec)?;
        Ok(SignedStakeKeyRegistration {
            data,
            sig,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyRegistration {
    pub stake_public_key: PublicKey,
}

impl StakeKeyRegistration {
    pub fn make_certificate(self, private_stake_key: &PrivateKey) -> Certificate {
        Certificate::StakeKeyRegistration(SignedStakeKeyRegistration {
            sig: private_stake_key.serialize_and_sign(&self),
            data: self
        })
    }
}

impl property::Serialize for StakeKeyRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_public_key.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakeKeyRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let stake_public_key = PublicKey::deserialize(&mut codec)?;
        Ok(StakeKeyRegistration {
            stake_public_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedStakePoolRegistration {
    pub data: StakePoolRegistration,
    //pub owner_sig: Signature,
    pub pool_sig: Signature,
}

impl property::Serialize for SignedStakePoolRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.data.serialize(&mut codec)?;
        //self.owner_sig.serialize(&mut codec)?;
        self.pool_sig.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for SignedStakePoolRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let data = StakePoolRegistration::deserialize(&mut codec)?;
        //let owner_sig = Signature::deserialize(&mut codec)?;
        let pool_sig = Signature::deserialize(&mut codec)?;
        Ok(SignedStakePoolRegistration {
            data,
            //owner_sig,
            pool_sig,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRegistration {
    pub pool_public_key: PublicKey,
    //pub owner: PublicKey, // FIXME: support list of owners
    // reward sharing params: cost, margin, pledged amount of stake
    // alternative stake key reward account
}

impl StakePoolRegistration {
    /// Create a certificate for this stake pool registration, signed
    /// by the pool's staking key and the owners.
    pub fn make_certificate(self, pool_private_key: &PrivateKey) -> Certificate {
        Certificate::StakePoolRegistration(SignedStakePoolRegistration {
            pool_sig: pool_private_key.serialize_and_sign(&self),
            data: self
        })
    }
}

impl property::Serialize for StakePoolRegistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.pool_public_key.serialize(&mut codec)?;
        //self.owner.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakePoolRegistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let pool_public_key = PublicKey::deserialize(&mut codec)?;
        //let owner = PublicKey::deserialize(&mut codec)?;
        Ok(StakePoolRegistration {
            pool_public_key,
            //owner,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedStakePoolRetirement {
    pub data: StakePoolRetirement,
    pub pool_sig: Signature,
}

impl property::Serialize for SignedStakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.data.serialize(&mut codec)?;
        self.pool_sig.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for SignedStakePoolRetirement {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let data = StakePoolRetirement::deserialize(&mut codec)?;
        let pool_sig = Signature::deserialize(&mut codec)?;
        Ok(SignedStakePoolRetirement {
            data,
            pool_sig,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolRetirement {
    pub pool_public_key: PublicKey,
    // TODO: add epoch when the retirement will take effect
}

impl StakePoolRetirement {
    /// Create a certificate for this stake pool retirement, signed
    /// by the pool's staking key.
    pub fn make_certificate(self, pool_private_key: &PrivateKey) -> Certificate {
        Certificate::StakePoolRetirement(SignedStakePoolRetirement {
            pool_sig: pool_private_key.serialize_and_sign(&self),
            data: self
        })
    }
}

impl property::Serialize for StakePoolRetirement {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.pool_public_key.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakePoolRetirement {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        let pool_public_key = PublicKey::deserialize(&mut codec)?;
        Ok(StakePoolRetirement {
            pool_public_key,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Certificate {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Certificate::StakePoolRegistration(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for StakePoolRegistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRegistration {
                pool_public_key: Arbitrary::arbitrary(g),
                //owner: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedStakePoolRegistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedStakePoolRegistration {
                data: Arbitrary::arbitrary(g),
                //owner_sig: Arbitrary::arbitrary(g),
                pool_sig: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRetirement {
                pool_public_key: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedStakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            SignedStakePoolRetirement {
                data: Arbitrary::arbitrary(g),
                pool_sig: Arbitrary::arbitrary(g),
            }
        }
    }
}
