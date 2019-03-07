use crate::key::{deserialize_public_key, serialize_public_key};
use crate::stake::StakePoolId;
use chain_core::property;
use chain_crypto::algorithms::vrf::vrf::{self, ProvenOutputSeed};
use chain_crypto::{Ed25519Extended, FakeMMM, PublicKey, SecretKey};

pub mod bft;
pub mod genesis;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeaderId(pub(crate) PublicKey<Ed25519Extended>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PublicLeader {
    None,
    Bft(LeaderId),
    GenesisPraos(PublicKey<FakeMMM>),
}

pub enum Leader {
    None,
    BftLeader(SecretKey<Ed25519Extended>),
    GenesisPraos(SecretKey<FakeMMM>, vrf::SecretKey, ProvenOutputSeed),
}

impl chain_core::property::LeaderId for LeaderId {}

impl From<StakePoolId> for LeaderId {
    fn from(id: StakePoolId) -> Self {
        LeaderId(id.0)
    }
}

impl From<PublicKey<Ed25519Extended>> for LeaderId {
    fn from(key: PublicKey<Ed25519Extended>) -> Self {
        LeaderId(key)
    }
}

impl property::Serialize for LeaderId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl property::Deserialize for LeaderId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        deserialize_public_key(reader).map(LeaderId)
    }
}

impl AsRef<PublicKey<Ed25519Extended>> for LeaderId {
    fn as_ref(&self) -> &PublicKey<Ed25519Extended> {
        &self.0
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for LeaderId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            use rand_chacha::ChaChaRng;
            use rand_core::SeedableRng;
            let mut seed = [0; 32];
            for byte in seed.iter_mut() {
                *byte = Arbitrary::arbitrary(g);
            }
            let mut rng = ChaChaRng::from_seed(seed);
            LeaderId(SecretKey::generate(&mut rng).to_public())
        }
    }
}
