use crate::key::{PrivateKey, PublicKey};
use crate::transaction::Value;
use chain_core::property;
use std::collections::{HashMap, HashSet};

// For each stake pool, the total stake value, and the value for the
// stake pool members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDistribution(pub HashMap<StakePoolId, (Value, HashMap<StakeKeyId, Value>)>);

impl StakeDistribution {
    pub fn empty() -> Self {
        StakeDistribution(HashMap::new())
    }

    /// Return the number of stake pools with non-zero stake.
    pub fn eligible_stake_pools(&self) -> usize {
        self.0.len()
    }

    /// Return the total stake held by the eligible stake pools.
    pub fn total_stake(&self) -> Value {
        self.0
            .iter()
            .map(|(_, (pool_stake, _))| pool_stake)
            .fold(Value(0), |sum, &x| sum + x)
    }

    /// Place the stake pools on the interval [0, total_stake) (sorted
    /// by ID), then return the ID of the one containing 'point'
    /// (which must be in the interval). This is used to randomly
    /// select a leader, taking stake into account.
    pub fn select_pool(&self, mut point: u64) -> Option<StakePoolId> {
        let mut pools_sorted: Vec<_> = self
            .0
            .iter()
            .map(|(pool_id, (pool_stake, _))| (pool_id, pool_stake))
            .collect();

        pools_sorted.sort();

        for (pool_id, pool_stake) in pools_sorted {
            if point < pool_stake.0 {
                return Some(pool_id.clone());
            }
            point -= pool_stake.0
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeKeyInfo {
    /// Current stake pool this key is a member of, if any.
    pub pool: Option<StakePoolId>,
    // - reward account
    // - registration deposit (if variable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakePoolInfo {
    //owners: HashSet<PublicKey>,
    pub members: HashSet<StakeKeyId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakeKeyId(pub PublicKey);

impl From<PublicKey> for StakeKeyId {
    fn from(key: PublicKey) -> Self {
        StakeKeyId(key)
    }
}

impl From<&PrivateKey> for StakeKeyId {
    fn from(key: &PrivateKey) -> Self {
        StakeKeyId(key.public())
    }
}

impl property::Serialize for StakeKeyId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.0.serialize(writer)
    }
}

impl property::Deserialize for StakeKeyId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Ok(StakeKeyId(PublicKey::deserialize(reader)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakePoolId(pub PublicKey);

impl From<&PrivateKey> for StakePoolId {
    fn from(key: &PrivateKey) -> Self {
        StakePoolId(key.public())
    }
}

impl property::Serialize for StakePoolId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.0.serialize(writer)
    }
}

impl property::Deserialize for StakePoolId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Ok(StakePoolId(PublicKey::deserialize(reader)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for StakeKeyId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakeKeyId(Arbitrary::arbitrary(g))
        }
    }

    impl Arbitrary for StakePoolId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolId(Arbitrary::arbitrary(g))
        }
    }
}
