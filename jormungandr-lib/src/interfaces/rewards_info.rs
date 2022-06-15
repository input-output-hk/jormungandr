use crate::{
    crypto::{account::Identifier, hash::Hash},
    interfaces::Value,
};
use chain_impl_mockchain::{block::Epoch, ledger::EpochRewardsInfo as EpochRewardsInfoStd};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct EpochRewardsInfo {
    epoch: Epoch,
    drawn: Value,
    fees: Value,
    treasury: Value,
    stake_pools: BTreeMap<Hash, (Value, Value)>,
    accounts: BTreeMap<Identifier, Value>,
}

impl EpochRewardsInfo {
    pub fn epoch(&self) -> Epoch {
        self.epoch
    }

    pub fn stake_pools(&self) -> &BTreeMap<Hash, (Value, Value)> {
        &self.stake_pools
    }

    pub fn accounts(&self) -> &BTreeMap<Identifier, Value> {
        &self.accounts
    }

    pub fn from(epoch: Epoch, eris: &EpochRewardsInfoStd) -> Self {
        Self {
            epoch,
            drawn: eris.drawn.into(),
            fees: eris.fees.into(),
            treasury: eris.treasury.into(),
            stake_pools: eris
                .stake_pools
                .iter()
                .map(|(k, (v1, v2))| (k.clone().into(), ((*v1).into(), (*v2).into())))
                .collect(),
            accounts: eris
                .accounts
                .iter()
                .map(|(k, v)| (k.clone().into(), (*v).into()))
                .collect(),
        }
    }
}

impl PartialEq for EpochRewardsInfo {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch
            && self.drawn == other.drawn
            && self.fees == other.fees
            && self.treasury == other.treasury
            && self.stake_pools == other.stake_pools
            && self.accounts == other.accounts
    }
}
impl Eq for EpochRewardsInfo {}
