use crate::account;
use crate::accounting::account::DelegationType;
use crate::certificate::PoolId;
use crate::{utxo, value::Value};
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
    pub to_pools: HashMap<PoolId, PoolStakeInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStakeInformation {
    pub total: PoolStakeTotal,
    pub stake_owners: PoolStakeDistribution,
}

impl PoolStakeInformation {
    pub fn add_value(&mut self, id: &account::Identifier, v: Value) {
        let account_stake = self
            .stake_owners
            .accounts
            .entry(id.clone())
            .or_insert(Value::zero());
        *account_stake = (*account_stake + v).expect("internal error: stake sum not valid");
        self.total.total_stake =
            (self.total.total_stake + v).expect("internal error: total amount of stake overflow");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStakeTotal {
    pub total_stake: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStakeDistribution {
    pub accounts: HashMap<account::Identifier, Value>,
}

impl PoolStakeDistribution {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn to_total(&self) -> Value {
        Value::sum(self.accounts.values().copied())
            .expect("cannot sum stake properly: internal error related to value")
    }
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
        Value::sum(self.to_pools.iter().map(|(_, pool)| pool.total.total_stake))
            .expect("cannot sum stake properly: internal error related to value")
    }

    pub fn get_stake_for(&self, poolid: &PoolId) -> Option<Value> {
        self.to_pools.get(poolid).map(|psd| psd.total.total_stake)
    }

    pub fn get_distribution(&self, pool_id: &PoolId) -> Option<&PoolStakeInformation> {
        self.to_pools.get(pool_id)
    }
}

fn assign_account_value(
    sd: &mut StakeDistribution,
    account_identifier: &account::Identifier,
    delegation_type: &DelegationType,
    value: Value,
) {
    match delegation_type {
        DelegationType::NonDelegated => sd.unassigned = (sd.unassigned + value).unwrap(),
        DelegationType::Full(ref pool_id) => {
            // if the pool exists, we add value to this pool distribution,
            // otherwise it get added to the dangling sum
            match sd.to_pools.get_mut(pool_id) {
                None => sd.dangling = (sd.dangling + value).unwrap(),
                Some(pool_info) => pool_info.add_value(&account_identifier, value),
            }
        }
        DelegationType::Ratio(dr) => {
            // is the ratio distribution is not correct, considered it unassigned, otherwise
            // separate the total in as many parts as pools, and try to assign from the first to the last,
            // the stake associated plus if there's any remaining from the division.
            if dr.is_valid() {
                assert!(dr.pools.len() > 0); // verified by is_valid
                let sin = value.split_in(dr.pools.len() as u32);
                let mut r = sin.remaining;
                for (pool_id, ratio) in dr.pools.iter() {
                    match sd.to_pools.get_mut(pool_id) {
                        None => sd.dangling = (sd.dangling + value).unwrap(),
                        Some(pool_info) => {
                            let pool_value = sin
                                .parts
                                .scale(*ratio as u32)
                                .expect("internal error: impossible overflow in ratio calculation");
                            let pool_value = (pool_value + r).unwrap();
                            r = Value::zero();
                            pool_info.add_value(&account_identifier, pool_value);
                        }
                    }
                    // if r is not zero already, then we fail to assign it to anything, so just considered it dangling
                    if r > Value::zero() {
                        sd.dangling = (sd.dangling + value).unwrap()
                    }
                }
            } else {
                sd.unassigned = (sd.unassigned + value).unwrap()
            }
        }
    }
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

    let p0 = PoolStakeInformation {
        total: PoolStakeTotal {
            total_stake: Value::zero(),
        },
        stake_owners: PoolStakeDistribution::new(),
    };

    let mut distribution = StakeDistribution {
        unassigned: Value::zero(),
        dangling: Value::zero(),
        to_pools: HashMap::from_iter(
            dstate
                .stake_pools
                .iter()
                .map(|(id, _)| (id.clone(), p0.clone())),
        ),
    };

    for (identifier, account_state) in accounts.iter() {
        assign_account_value(
            &mut distribution,
            identifier,
            &account_state.delegation(),
            account_state.value(),
        )
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
                let identifier = account_key.clone().into();
                // is there an account linked to this
                match accounts.get_state(&identifier) {
                    Err(_) => panic!("internal error: group's account should always be created"),
                    Ok(st) => assign_account_value(
                        &mut distribution,
                        &identifier,
                        &st.delegation(),
                        output.value,
                    ),
                }
            }
            Kind::Single(_) => {
                distribution.unassigned = (distribution.unassigned + output.value).unwrap();
            }
        }
    }

    distribution
}
