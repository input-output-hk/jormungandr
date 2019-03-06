use crate::key::{deserialize_public_key, serialize_public_key};
use crate::ledger::Ledger;
use crate::value::Value;
use chain_addr::Kind;
use chain_core::property;
use chain_crypto::{Ed25519, PublicKey, SecretKey};
use std::collections::{HashMap, HashSet};

/// A structure that keeps track of stake keys and stake pools.
#[derive(Debug)]
pub struct DelegationState {
    stake_keys: HashMap<StakeKeyId, StakeKeyInfo>,
    stake_pools: HashMap<StakePoolId, StakePoolInfo>,
}

impl DelegationState {
    pub fn new(
        initial_stake_pools: HashSet<StakePoolId>,
        initial_stake_keys: HashMap<StakeKeyId, Option<StakePoolId>>,
    ) -> Self {
        let mut stake_pools: HashMap<StakePoolId, StakePoolInfo> = initial_stake_pools
            .into_iter()
            .map(|pool_id| {
                (
                    pool_id,
                    StakePoolInfo {
                        members: HashSet::new(),
                    },
                )
            })
            .collect();

        let mut stake_keys = HashMap::new();
        for (stake_key_id, pool_id) in initial_stake_keys {
            if let Some(pool_id) = &pool_id {
                if let Some(pool) = stake_pools.get_mut(&pool_id) {
                    pool.members.insert(stake_key_id.clone());
                } else {
                    panic!("Pool '{:?}' does not exist.", pool_id)
                }
            }
            stake_keys.insert(stake_key_id, StakeKeyInfo { pool: pool_id });
        }

        DelegationState {
            stake_keys,
            stake_pools,
        }
    }

    pub fn get_stake_distribution(&self, ledger: &Ledger) -> StakeDistribution {
        let mut dist = HashMap::new();

        for (ptr, output) in ledger.unspent_outputs.iter() {
            assert_eq!(ptr.value, output.1);

            // We're only interested in "group" addresses
            // (i.e. containing a spending key and a stake key).
            if let Kind::Group(_spending_key, stake_key) = output.0.kind() {
                // Grmbl.
                let stake_key = stake_key.clone().into();

                // Do we have a stake key for this spending key?
                if let Some(stake_key_info) = self.stake_keys.get(&stake_key) {
                    // Is this stake key a member of a stake pool?
                    if let Some(pool_id) = &stake_key_info.pool {
                        let pool = &self.stake_pools[pool_id];
                        debug_assert!(pool.members.contains(&stake_key));
                        let stake_pool_dist =
                            dist.entry(pool_id.clone())
                                .or_insert_with(|| PoolStakeDistribution {
                                    total_stake: Value(0),
                                    member_stake: HashMap::new(),
                                });
                        stake_pool_dist.total_stake += ptr.value;
                        let member_dist = stake_pool_dist
                            .member_stake
                            .entry(stake_key.clone())
                            .or_insert_with(|| Value(0));
                        *member_dist += ptr.value;
                    }
                }
            }
        }

        StakeDistribution(dist)
    }

    pub fn nr_stake_keys(&self) -> usize {
        self.stake_keys.len()
    }

    pub fn stake_key_exists(&self, stake_key_id: &StakeKeyId) -> bool {
        self.stake_keys.contains_key(stake_key_id)
    }

    pub fn register_stake_key(&mut self, stake_key_id: StakeKeyId) {
        let inserted = !self
            .stake_keys
            .insert(stake_key_id, StakeKeyInfo { pool: None })
            .is_some();
        assert!(inserted);
    }

    pub fn deregister_stake_key(&mut self, stake_key_id: &StakeKeyId) {
        let stake_key_info = self.stake_keys.remove(&stake_key_id).unwrap();

        // Remove this stake key from its pool, if any.
        if let Some(pool_id) = stake_key_info.pool {
            self.stake_pools
                .get_mut(&pool_id)
                .unwrap()
                .members
                .remove(&stake_key_id);
        }
    }

    pub fn nr_stake_pools(&self) -> usize {
        self.stake_pools.len()
    }

    pub fn stake_pool_exists(&self, pool_id: &StakePoolId) -> bool {
        self.stake_pools.contains_key(pool_id)
    }

    pub fn register_stake_pool(&mut self, pool_id: StakePoolId) {
        assert!(!self.stake_pools.contains_key(&pool_id));
        self.stake_pools.insert(
            pool_id,
            StakePoolInfo {
                //owners: new_stake_pool.owners
                members: HashSet::new(),
            },
        );
    }

    pub fn deregister_stake_pool(&mut self, pool_id: &StakePoolId) {
        let pool_info = self.stake_pools.remove(pool_id).unwrap();

        // Remove all pool members.
        for member in pool_info.members {
            let stake_key_info = self.stake_keys.get_mut(&member).unwrap();
            assert_eq!(stake_key_info.pool.as_ref().unwrap(), pool_id);
            stake_key_info.pool = None;
        }
    }

    pub fn nr_pool_members(&self, pool_id: StakePoolId) -> usize {
        self.stake_pools[&pool_id].members.len()
    }

    pub fn delegate_stake(&mut self, stake_key_id: StakeKeyId, pool_id: StakePoolId) {
        let stake_key = self.stake_keys.get_mut(&stake_key_id).unwrap();

        // If this is a redelegation, remove the stake key from its previous pool.
        if let Some(prev_stake_pool) = &stake_key.pool {
            let removed = self
                .stake_pools
                .get_mut(&prev_stake_pool)
                .unwrap()
                .members
                .remove(&stake_key_id);
            assert!(removed);
        }

        let stake_pool = self.stake_pools.get_mut(&pool_id).unwrap();
        stake_key.pool = Some(pool_id);
        stake_pool.members.insert(stake_key_id);
    }
}

/// For each stake pool, the total stake value, and the value for the
/// stake pool members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDistribution(pub HashMap<StakePoolId, PoolStakeDistribution>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStakeDistribution {
    pub total_stake: Value,
    /// Stake per member. Non-zero stakes only.
    pub member_stake: HashMap<StakeKeyId, Value>,
}

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
            .map(|(_, pool)| pool.total_stake)
            .fold(Value(0), |sum, x| sum + x)
    }

    /// Place the stake pools on the interval [0, total_stake) (sorted
    /// by ID), then return the ID of the one containing 'point'
    /// (which must be in the interval). This is used to randomly
    /// select a leader, taking stake into account.
    pub fn select_pool(&self, mut point: u64) -> Option<StakePoolId> {
        let mut pools_sorted: Vec<_> = self
            .0
            .iter()
            .map(|(pool_id, pool)| (pool_id, pool.total_stake))
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
pub struct StakeKeyId(PublicKey<Ed25519>);

impl From<PublicKey<Ed25519>> for StakeKeyId {
    fn from(key: PublicKey<Ed25519>) -> Self {
        StakeKeyId(key)
    }
}

impl From<&SecretKey<Ed25519>> for StakeKeyId {
    fn from(key: &SecretKey<Ed25519>) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StakePoolId(pub PublicKey<Ed25519>);

impl From<&SecretKey<Ed25519>> for StakePoolId {
    fn from(key: &SecretKey<Ed25519>) -> Self {
        StakePoolId(key.to_public())
    }
}

impl property::Serialize for StakePoolId {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        serialize_public_key(&self.0, writer)
    }
}

impl property::Deserialize for StakePoolId {
    type Error = std::io::Error;
    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        deserialize_public_key(reader).map(StakePoolId)
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

    impl Arbitrary for StakePoolId {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            StakePoolId::from(&crate::key::test::arbitrary_secret_key(g))
        }
    }
}
