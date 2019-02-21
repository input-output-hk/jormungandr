use crate::key::*;
use chain_core::property;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Certificate {
    StakeKeyRegistration(Signed<StakeKeyRegistration>),
    StakeKeyDeregistration(Signed<StakeKeyDeregistration>),
    StakeDelegation(Signed<StakeDelegation>),
    StakePoolRegistration(Signed<StakePoolRegistration>),
    StakePoolRetirement(Signed<StakePoolRetirement>),
}

pub const TAG_STAKE_KEY_REGISTRATION: u8 = 1;
pub const TAG_STAKE_KEY_DEREGISTRATION: u8 = 2;
pub const TAG_STAKE_DELEGATION: u8 = 3;
pub const TAG_STAKE_POOL_REGISTRATION: u8 = 4;
pub const TAG_STAKE_POOL_RETIREMENT: u8 = 5;

impl property::Serialize for Certificate {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        match self {
            Certificate::StakeKeyRegistration(signed) => {
                codec.put_u8(TAG_STAKE_KEY_REGISTRATION)?;
                signed.serialize(&mut codec)
            }
            Certificate::StakeKeyDeregistration(signed) => {
                codec.put_u8(TAG_STAKE_KEY_DEREGISTRATION)?;
                signed.serialize(&mut codec)
            }
            Certificate::StakeDelegation(signed) => {
                codec.put_u8(TAG_STAKE_DELEGATION)?;
                signed.serialize(&mut codec)
            }
            Certificate::StakePoolRegistration(signed) => {
                codec.put_u8(TAG_STAKE_POOL_REGISTRATION)?;
                signed.serialize(&mut codec)
            }
            Certificate::StakePoolRetirement(signed) => {
                codec.put_u8(TAG_STAKE_POOL_RETIREMENT)?;
                signed.serialize(&mut codec)
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
            TAG_STAKE_KEY_REGISTRATION => Ok(Certificate::StakeKeyRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            TAG_STAKE_KEY_DEREGISTRATION => Ok(Certificate::StakeKeyDeregistration(
                Signed::deserialize(&mut codec)?,
            )),
            TAG_STAKE_DELEGATION => Ok(Certificate::StakeDelegation(Signed::deserialize(
                &mut codec,
            )?)),
            TAG_STAKE_POOL_REGISTRATION => Ok(Certificate::StakePoolRegistration(
                Signed::deserialize(&mut codec)?,
            )),
            TAG_STAKE_POOL_RETIREMENT => Ok(Certificate::StakePoolRetirement(Signed::deserialize(
                &mut codec,
            )?)),
            n => panic!("Unrecognized certificate tag {}.", n), // FIXME: return Error
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed<T> {
    pub data: T,
    pub sig: Signature,
}

impl<T: property::Serialize> property::Serialize for Signed<T>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.data.serialize(&mut codec)?;
        self.sig.serialize(&mut codec)?;
        Ok(())
    }
}

impl<T: property::Deserialize> property::Deserialize for Signed<T>
where
    std::io::Error: From<T::Error>,
{
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(Signed {
            data: T::deserialize(&mut codec)?,
            sig: Signature::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyRegistration {
    pub stake_public_key: PublicKey,
}

impl StakeKeyRegistration {
    pub fn make_certificate(self, stake_private_key: &PrivateKey) -> Certificate {
        Certificate::StakeKeyRegistration(Signed {
            sig: stake_private_key.serialize_and_sign(&self),
            data: self,
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
        Ok(StakeKeyRegistration {
            stake_public_key: PublicKey::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyDeregistration {
    pub stake_public_key: PublicKey,
}

impl StakeKeyDeregistration {
    pub fn make_certificate(self, stake_private_key: &PrivateKey) -> Certificate {
        Certificate::StakeKeyDeregistration(Signed {
            sig: stake_private_key.serialize_and_sign(&self),
            data: self,
        })
    }
}

impl property::Serialize for StakeKeyDeregistration {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_public_key.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakeKeyDeregistration {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakeKeyDeregistration {
            stake_public_key: PublicKey::deserialize(&mut codec)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDelegation {
    pub stake_public_key: PublicKey,
    pub pool_public_key: PublicKey,
}

impl StakeDelegation {
    pub fn make_certificate(self, stake_private_key: &PrivateKey) -> Certificate {
        // FIXME: "It must be signed by sks_source, and that key must
        // be included in the witness." - why?
        Certificate::StakeDelegation(Signed {
            sig: stake_private_key.serialize_and_sign(&self),
            data: self,
        })
    }
}

impl property::Serialize for StakeDelegation {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(writer);
        self.stake_public_key.serialize(&mut codec)?;
        self.pool_public_key.serialize(&mut codec)?;
        Ok(())
    }
}

impl property::Deserialize for StakeDelegation {
    type Error = std::io::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::from(reader);
        Ok(StakeDelegation {
            stake_public_key: PublicKey::deserialize(&mut codec)?,
            pool_public_key: PublicKey::deserialize(&mut codec)?,
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
        Certificate::StakePoolRegistration(Signed {
            sig: pool_private_key.serialize_and_sign(&self),
            data: self,
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
        Ok(StakePoolRegistration {
            pool_public_key: PublicKey::deserialize(&mut codec)?,
            // owner: PublicKey::deserialize(&mut codec)?,
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
        Certificate::StakePoolRetirement(Signed {
            sig: pool_private_key.serialize_and_sign(&self),
            data: self,
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
        Ok(StakePoolRetirement {
            pool_public_key: PublicKey::deserialize(&mut codec)?,
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

    impl<T: Arbitrary> Arbitrary for Signed<T> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Signed {
                data: Arbitrary::arbitrary(g),
                sig: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakeKeyRegistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyRegistration {
                stake_public_key: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for StakeKeyDeregistration {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyDeregistration {
                stake_public_key: Arbitrary::arbitrary(g),
            }
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

    impl Arbitrary for StakePoolRetirement {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolRetirement {
                pool_public_key: Arbitrary::arbitrary(g),
            }
        }
    }
}
