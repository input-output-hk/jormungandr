use crate::block::{Block, Header, Proof};
use crate::key::{deserialize_public_key, serialize_public_key};
use crate::{
    leadership::{self, Error, ErrorKind, Verification},
    state::State,
};
use chain_core::property::{self, LeaderSelection};
use chain_crypto::{Ed25519Extended, PublicKey, SecretKey};
use std::rc::Rc;

/// cryptographic signature algorithm used for the BFT leadership
/// protocol.
#[allow(non_camel_case_types)]
pub type SIGNING_ALGORITHM = Ed25519Extended;

/// BFT Leader signing key
pub type SigningKey = SecretKey<SIGNING_ALGORITHM>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LeaderId(pub(crate) PublicKey<SIGNING_ALGORITHM>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BftRoundRobinIndex(u64);

/// The BFT Leader selection is based on a round robin of the expected leaders
#[derive(Debug)]
pub struct BftLeaderSelection {
    pub(crate) leaders: Rc<Vec<LeaderId>>,
}

impl BftLeaderSelection {
    /// Create a new BFT leadership
    pub fn new(state: &State) -> Option<Self> {
        if state.settings.bft_leaders.len() == 0 {
            return None;
        }

        Some(BftLeaderSelection {
            leaders: state.settings.bft_leaders.clone(),
        })
    }

    #[inline]
    pub fn number_of_leaders(&self) -> usize {
        self.leaders.len()
    }

    #[inline]
    fn offset(&self, block_number: u64) -> BftRoundRobinIndex {
        let max = self.number_of_leaders() as u64;
        BftRoundRobinIndex((block_number % max) as u64)
    }

    pub(crate) fn verify(&self, block_header: &Header) -> Verification {
        match &block_header.proof() {
            Proof::Bft(bft_proof) => match self.get_leader_at(*block_header.block_date()) {
                Ok(leadership::LeaderId::Bft(leader_at)) => {
                    if bft_proof.leader_id != leader_at {
                        Verification::Failure(Error::new(ErrorKind::InvalidLeader))
                    } else {
                        Verification::Success
                    }
                }
                Err(error) => Verification::Failure(error),
                Ok(_) => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
            },
            _ => Verification::Failure(Error::new(ErrorKind::InvalidLeaderSignature)),
        }
    }
}

impl LeaderSelection for BftLeaderSelection {
    type Block = Block;
    type Error = Error;
    type LeaderId = leadership::LeaderId;

    #[inline]
    fn get_leader_at(
        &self,
        date: <Self::Block as property::Block>::Date,
    ) -> Result<Self::LeaderId, Self::Error> {
        let BftRoundRobinIndex(ofs) = self.offset(date.slot_id as u64);
        Ok(leadership::LeaderId::Bft(
            self.leaders[ofs as usize].clone(),
        ))
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

impl AsRef<[u8]> for LeaderId {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl From<PublicKey<SIGNING_ALGORITHM>> for LeaderId {
    fn from(v: PublicKey<SIGNING_ALGORITHM>) -> Self {
        LeaderId(v)
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
