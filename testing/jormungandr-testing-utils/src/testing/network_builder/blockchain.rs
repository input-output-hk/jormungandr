use super::{ExternalWalletTemplate, LegacyWalletTemplate, NodeAlias, WalletAlias, WalletTemplate};
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_addr::Discrimination;
use chain_impl_mockchain::{fee::LinearFee, testing::scenario::template::VotePlanDef};
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
    committees: Vec<WalletAlias>,
    vote_plans: Vec<VotePlanDef>,
    legacy_wallets: Vec<LegacyWalletTemplate>,
    external_wallets: Vec<ExternalWalletTemplate>,
    wallets: HashMap<WalletAlias, WalletTemplate>,
    kes_update_speed: KESUpdateSpeed,
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    linear_fee: LinearFee,
    discrimination: Discrimination,
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
            committees: Vec::new(),
            vote_plans: Vec::new(),
            legacy_wallets: Vec::new(),
            external_wallets: Vec::new(),
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            consensus_genesis_praos_active_slot_coeff,
            linear_fee: LinearFee::new(1, 1, 1),
            discrimination: Discrimination::Test
        }
    }

    pub fn committees(&self) -> Vec<WalletAlias> {
        self.committees.clone()
    }

    pub fn external_wallets(&self) -> Vec<ExternalWalletTemplate> {
        self.external_wallets.clone()
    }

    pub fn legacy_wallets(&self) -> Vec<LegacyWalletTemplate> {
        self.legacy_wallets.clone()
    }

    pub fn vote_plans(&self) -> Vec<VotePlanDef> {
        self.vote_plans.clone()
    }

    pub fn linear_fee(&self) -> LinearFee {
        self.linear_fee
    }

    pub fn set_linear_fee(&mut self, linear_fee: LinearFee) {
        self.linear_fee = linear_fee;
    }
    
    pub fn discrimination(&self) -> Discrimination {
        self.discrimination
    }

    pub fn set_discrimination(&mut self, discrimination: Discrimination) {
        self.discrimination = discrimination;
    }

    pub fn vote_plan(&self, alias: &str) -> Option<VotePlanDef> {
        self.vote_plans()
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
    }

    pub fn add_committee<S: Into<NodeAlias>>(&mut self, alias: S) {
        self.committees.push(alias.into())
    }

    pub fn add_legacy_wallet(&mut self, legacy_wallet: LegacyWalletTemplate) {
        self.legacy_wallets.push(legacy_wallet);
    }

    pub fn add_vote_plan(&mut self, vote_plan_template: VotePlanDef) {
        self.vote_plans.push(vote_plan_template);
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

    pub fn leaders(&self) -> impl Iterator<Item = &NodeAlias> {
        self.leaders.iter()
    }

    pub fn wallets(&self) -> impl Iterator<Item = &WalletTemplate> {
        self.wallets.values()
    }
}
