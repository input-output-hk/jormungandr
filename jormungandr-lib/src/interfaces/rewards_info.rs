use crate::{
    crypto::{account::Identifier, hash::Hash},
    interfaces::Value,
};
use chain_impl_mockchain::block::Epoch;
use chain_impl_mockchain::ledger::EpochRewardsInfo as EpochRewardsInfoStd;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
pub struct EpochRewardsInfo {
    epoch: Epoch,
    drawn: Value,
    fees: Value,
    treasury: Value,
    stake_pools: BTreeMap<Hash, (Value, Value)>,
    accounts: BTreeMap<Identifier, Value>,
}

impl EpochRewardsInfo {
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
