use crate::leadership::bft::LeaderId;
use chain_crypto::{Ed25519Extended, SecretKey, KeyPair};
use std::fmt::{self, Debug};
use quickcheck::{Arbitrary,Gen};


#[derive(Clone)]
pub struct LeaderPair {
    leader_id: LeaderId,
    leader_key: SecretKey<Ed25519Extended>,
}

impl PartialEq<LeaderPair> for LeaderPair {
    fn eq(&self, other: &LeaderPair) -> bool {
        self.id() == other.id()
    }
}

impl Debug for LeaderPair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LeaderPair")
            .field("proposal", &self.id())
            .finish()
    }
}

impl LeaderPair {
    pub fn new(leader_id: LeaderId, leader_key: SecretKey<Ed25519Extended>) -> Self {
        LeaderPair {
            leader_id,
            leader_key,
        }
    }

    pub fn id(&self) -> LeaderId {
        self.leader_id.clone()
    }

    pub fn key(&self) -> SecretKey<Ed25519Extended> {
        self.leader_key.clone()
    }
}

impl Arbitrary for LeaderPair {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        LeaderPair{
            leader_id: LeaderId::arbitrary(g),
            leader_key: KeyPair::<Ed25519Extended>::arbitrary(g).private_key().clone(),
        }
    }
}
