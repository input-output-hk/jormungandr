use crate::account;
use crate::{utxo, value::Value};
use crate::certificate::PoolId;
use chain_addr::{Address, Kind};
use std::collections::HashMap;

use super::delegation::DelegationState;

/// Stake distribution at a given time.
///
/// Mainly containing the value associated with each pool,
/// but in future could also contains:
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeDistribution {
    // single address values
    pub unassigned: Value,
    // group or account that doesn't have a valid stake pool assigned
    pub dangling: Value,
    /// For each stake pool, the total stake value, and the value for the
    /// stake pool members.
    pub to_pools: HashMap<PoolId, PoolStakeDistribution>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStakeDistribution {
    pub total_stake: Value,
}

impl StakeDistribution {
    pub fn empty() -> Self {
        StakeDistribution {
            unassigned: Value::zero(),
            dangling: Value::zero(),
            to_pools: HashMap::new(),
        }
    }

    /// Return the number of stake pools with non-zero stake.
    pub fn eligible_stake_pools(&self) -> usize {
        self.to_pools.len()
    }

    /// Return the total stake held by the eligible stake pools.
    pub fn total_stake(&self) -> Value {
        self.to_pools
            .iter()
            .map(|(_, pool)| pool.total_stake)
            .fold(Value::zero(), |sum, x| (sum + x).unwrap())
    }

    pub fn get_stake_for(&self, poolid: &PoolId) -> Option<Value> {
        self.to_pools.get(poolid).map(|psd| psd.total_stake)
    }

    pub fn get_distribution(&self, pool_id: &PoolId) -> Option<&PoolStakeDistribution> {
        self.to_pools.get(pool_id)
    }
}

pub fn distribution_add(p: &mut PoolStakeDistribution, v: Value) {
    p.total_stake = (p.total_stake + v).expect("internal error: total amount of stake overflow")
}

/// Calculate the Stake Distribution where the source of stake is coming from utxos and accounts,
/// and where the main targets is to calculate each value associated with *known* stake pools.
///
/// Everything that is linked to a stake pool that doesn't exist, will be added to dangling stake,
/// whereas all the utxo / accounts that doesn't have any delegation setup, will be counted towards
/// the unassigned stake.
pub fn get_distribution(
    accounts: &account::Ledger,
    dstate: &DelegationState,
    utxos: &utxo::Ledger<Address>,
) -> StakeDistribution {
    use std::iter::FromIterator;

    let p0 = PoolStakeDistribution {
        total_stake: Value::zero(),
    };
    let mut dist = HashMap::from_iter(dstate.stake_pools.iter().map(|(id, _)| (id.clone(), p0)));
    let mut unassigned = Value::zero();
    let mut dangling = Value::zero();

    for (_, account_state) in accounts.iter() {
        match account_state.delegation() {
            None => unassigned = (unassigned + account_state.value()).unwrap(),
            Some(pool_id) => {
                // if the pool exists, we add value to this pool distribution,
                // otherwise it get added to the dangling pool
                dist.get_mut(pool_id).map_or_else(
                    || dangling = (dangling + account_state.value()).unwrap(),
                    |v| distribution_add(v, account_state.value()),
                )
            }
        }
    }

    for output in utxos.values() {
        // We're only interested in "group" addresses
        // (i.e. containing a spending key and a stake key).
        match output.address.kind() {
            Kind::Account(_) | Kind::Multisig(_) => {
                // single or multisig account are not present in utxos
                panic!("internal error: accounts in utxo")
            }
            Kind::Group(_spending_key, account_key) => {
                // is there an account linked to this
                match accounts.get_state(&account_key.clone().into()) {
                    Err(_) => panic!("internal error: group's account should always be created"),
                    Ok(st) => {
                        // Is this stake key a member of a stake pool?
                        if let Some(pool_id) = &st.delegation() {
                            dist.get_mut(pool_id).map_or_else(
                                || dangling = (dangling + output.value).unwrap(),
                                |v| distribution_add(v, output.value),
                            );
                        }
                    }
                }
            }
            Kind::Single(_) => {
                unassigned = (unassigned + output.value).unwrap();
            }
        }
    }

    StakeDistribution {
        unassigned: unassigned,
        dangling: dangling,
        to_pools: dist,
    }
}
