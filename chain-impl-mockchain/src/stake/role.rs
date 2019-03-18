use crate::{
    key::{deserialize_public_key, serialize_public_key},
    leadership::genesis::GenesisPraosId,
};
use chain_core::property;
use chain_crypto::{algorithms::vrf::vrf, Ed25519Extended, FakeMMM, PublicKey, SecretKey};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyInfo {
    /// Current stake pool this key is a member of, if any.
    pub pool: Option<GenesisPraosId>,
    // - reward account
    // - registration deposit (if variable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolInfo {
    pub pool_id: GenesisPraosId,
    pub owner: StakeKeyId,
    pub members: HashSet<StakeKeyId>,
    pub vrf_public_key: vrf::PublicKey,
    pub kes_public_key: PublicKey<FakeMMM>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakeKeyId(pub(crate) PublicKey<Ed25519Extended>);

impl From<PublicKey<Ed25519Extended>> for StakeKeyId {
    fn from(key: PublicKey<Ed25519Extended>) -> Self {
        StakeKeyId(key)
    }
}

impl From<&SecretKey<Ed25519Extended>> for StakeKeyId {
    fn from(key: &SecretKey<Ed25519Extended>) -> Self {
        StakeKeyId(key.to_public())
    }
}

impl property::Serialize for StakeKeyId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl property::Deserialize for StakeKeyId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        deserialize_public_key(reader).map(StakeKeyId)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for StakeKeyId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyId::from(&crate::key::test::arbitrary_secret_key(g))
        }
    }
}
