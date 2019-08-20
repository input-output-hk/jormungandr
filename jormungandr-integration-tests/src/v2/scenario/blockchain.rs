use crate::v2::scenario::{ConsensusVersion, NodeAlias, Wallet, WalletAlias};
use jormungandr_lib::interfaces::{NumberOfSlotsPerEpoch, SlotDuration};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Blockchain {
    consensus: ConsensusVersion,
    slots_per_epoch: NumberOfSlotsPerEpoch,
    slot_duration: SlotDuration,
    leaders: Vec<NodeAlias>,
    wallets: HashMap<WalletAlias, Wallet>,
}

impl Blockchain {
    pub fn new(
        consensus: ConsensusVersion,
        slots_per_epoch: NumberOfSlotsPerEpoch,
        slot_duration: SlotDuration,
    ) -> Self {
        Blockchain {
            consensus,
            leaders: Vec::new(),
            wallets: HashMap::new(),
            slots_per_epoch,
            slot_duration,
        }
    }

    pub fn add_leader<S: Into<NodeAlias>>(&mut self, alias: S) {
        self.leaders.push(alias.into())
    }

    pub fn add_wallet(&mut self, wallet: Wallet) {
        self.wallets.insert(wallet.alias().clone(), wallet);
    }

    pub fn consensus(&self) -> &ConsensusVersion {
        &self.consensus
    }

    pub fn slots_per_epoch(&self) -> &NumberOfSlotsPerEpoch {
        &self.slots_per_epoch
    }

    pub fn slot_duration(&self) -> &SlotDuration {
        &self.slot_duration
    }

    pub fn leaders<'a>(&'a self) -> impl Iterator<Item = &'a NodeAlias> {
        self.leaders.iter()
    }

    pub fn wallets<'a>(&'a self) -> impl Iterator<Item = &'a Wallet> {
        self.wallets.values()
    }
}
