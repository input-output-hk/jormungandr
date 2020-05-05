use super::{NodeAlias, WalletAlias, WalletTemplate};
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, KESUpdateSpeed, NumberOfSlotsPerEpoch, SlotDuration,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Blockchain {
    consensus: ConsensusVersion,
    slots_per_epoch: NumberOfSlotsPerEpoch,
    slot_duration: SlotDuration,
    leaders: Vec<NodeAlias>,
    wallets: HashMap<WalletAlias, WalletTemplate>,
    kes_update_speed: KESUpdateSpeed,
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
}

impl Blockchain {
    pub fn new(
        consensus: ConsensusVersion,
        slots_per_epoch: NumberOfSlotsPerEpoch,
        slot_duration: SlotDuration,
        kes_update_speed: KESUpdateSpeed,
        consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    ) -> Self {
        Blockchain {
            consensus,
            leaders: Vec::new(),
            wallets: HashMap::new(),
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            consensus_genesis_praos_active_slot_coeff,
        }
    }

    pub fn add_leader<S: Into<NodeAlias>>(&mut self, alias: S) {
        self.leaders.push(alias.into())
    }

    pub fn add_wallet(&mut self, wallet: WalletTemplate) {
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

    pub fn kes_update_speed(&self) -> &KESUpdateSpeed {
        &self.kes_update_speed
    }

    pub fn consensus_genesis_praos_active_slot_coeff(&self) -> &ActiveSlotCoefficient {
        &self.consensus_genesis_praos_active_slot_coeff
    }

    pub fn leaders<'a>(&'a self) -> impl Iterator<Item = &'a NodeAlias> {
        self.leaders.iter()
    }

    pub fn wallets<'a>(&'a self) -> impl Iterator<Item = &'a WalletTemplate> {
        self.wallets.values()
    }
}
