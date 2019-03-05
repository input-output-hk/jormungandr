use crate::key::{PrivateKey, PublicKey};
use crate::stake::StakePoolId;
use chain_core::property;
use ouroboros_praos::vrf::{ProvenOutputSeed, SecretKey};

pub mod bft;
pub mod genesis;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeaderId(pub PublicKey);

pub enum Leader {
    BftLeader(PrivateKey),
    GenesisPraos(SecretKey, PrivateKey, ProvenOutputSeed),
}

impl chain_core::property::LeaderId for LeaderId {}

impl From<StakePoolId> for LeaderId {
    fn from(id: StakePoolId) -> Self {
        LeaderId(id.0)
    }
}

impl From<&PrivateKey> for LeaderId {
    fn from(key: &PrivateKey) -> Self {
        LeaderId(key.public())
    }
}
impl From<PublicKey> for LeaderId {
    fn from(key: PublicKey) -> Self {
        LeaderId(key)
    }
}

impl property::Serialize for LeaderId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.0.serialize(writer)
    }
}

impl property::Deserialize for LeaderId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Ok(LeaderId(PublicKey::deserialize(reader)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for LeaderId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            LeaderId(Arbitrary::arbitrary(g))
        }
    }
}
