use crate::{stake::StakePoolId, utxo, value::Value};
use chain_addr::{Address, Kind};
use std::collections::HashMap;

use super::delegation::DelegationState;
use super::role::StakeKeyId;

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
            .fold(Value::zero(), |sum, x| (sum + x).unwrap())
    }

    pub fn get_stake_for(&self, poolid: &StakePoolId) -> Option<Value> {
        self.0.get(poolid).map(|psd| psd.total_stake)
    }

    pub fn get_distribution(&self, stake_pool_id: &StakePoolId) -> Option<&PoolStakeDistribution> {
        self.0.get(stake_pool_id)
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

pub fn get_distribution(
    dstate: &DelegationState,
    utxos: &utxo::Ledger<Address>,
) -> StakeDistribution {
    let mut dist = HashMap::new();

    for output in utxos.values() {
        // We're only interested in "group" addresses
        // (i.e. containing a spending key and a stake key).
        if let Kind::Group(_spending_key, stake_key) = output.address.kind() {
            // Grmbl.
            let stake_key = stake_key.clone().into();

            // Do we have a stake key for this spending key?
            if let Some(stake_key_info) = dstate.stake_keys.lookup(&stake_key) {
                // Is this stake key a member of a stake pool?
                if let Some(pool_id) = &stake_key_info.pool {
                    let stake_pool_dist =
                        dist.entry(pool_id.clone())
                            .or_insert_with(|| PoolStakeDistribution {
                                total_stake: Value(0),
                                member_stake: HashMap::new(),
                            });
                    // note: unwrap should be safe, the system should have a total less than overflow
                    stake_pool_dist.total_stake =
                        (stake_pool_dist.total_stake + output.value).unwrap();

                    let member_dist = stake_pool_dist
                        .member_stake
                        .entry(stake_key.clone())
                        .or_insert_with(|| Value::zero());
                    *member_dist = (*member_dist + output.value).unwrap();
                }
            }
        }
    }

    StakeDistribution(dist)
}
