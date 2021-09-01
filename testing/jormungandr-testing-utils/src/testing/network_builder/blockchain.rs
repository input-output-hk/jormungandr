use super::{ExternalWalletTemplate, LegacyWalletTemplate, NodeAlias, WalletAlias, WalletTemplate};
use chain_addr::Discrimination;
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::{fee::LinearFee, testing::scenario::template::VotePlanDef};
use jormungandr_lib::interfaces::CommitteeIdDef;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, BlockContentMaxSize, KesUpdateSpeed, NumberOfSlotsPerEpoch, SlotDuration,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Blockchain {
    consensus: ConsensusVersion,
    slots_per_epoch: NumberOfSlotsPerEpoch,
    slot_duration: SlotDuration,
    leaders: Vec<NodeAlias>,
    committees: Vec<WalletAlias>,
    external_committees: Vec<CommitteeIdDef>,
    vote_plans: Vec<VotePlanDef>,
    legacy_wallets: Vec<LegacyWalletTemplate>,
    external_wallets: Vec<ExternalWalletTemplate>,
    wallets: HashMap<WalletAlias, WalletTemplate>,
    kes_update_speed: KesUpdateSpeed,
    block_content_max_size: BlockContentMaxSize,
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    tx_max_expiry_epochs: Option<u8>,
    linear_fee: LinearFee,
    discrimination: Discrimination,
}

impl Blockchain {
    pub fn new(
        consensus: ConsensusVersion,
        slots_per_epoch: NumberOfSlotsPerEpoch,
        slot_duration: SlotDuration,
        kes_update_speed: KesUpdateSpeed,
        consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    ) -> Self {
        Blockchain {
            consensus,
            leaders: Vec::new(),
            wallets: HashMap::new(),
            committees: Vec::new(),
            external_committees: Vec::new(),
            vote_plans: Vec::new(),
            legacy_wallets: Vec::new(),
            external_wallets: Vec::new(),
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            consensus_genesis_praos_active_slot_coeff,
            linear_fee: LinearFee::new(1, 1, 1),
            discrimination: Discrimination::Test,
            block_content_max_size: 102400.into(),
            tx_max_expiry_epochs: None,
        }
    }

    pub fn committees(&self) -> Vec<WalletAlias> {
        self.committees.clone()
    }

    pub fn external_committees(&self) -> Vec<CommitteeIdDef> {
        self.external_committees.clone()
    }

    pub fn tx_max_expiry_epochs(&self) -> Option<u8> {
        self.tx_max_expiry_epochs
    }

    pub fn set_tx_max_expiry_epochs(&mut self, tx_max_expiry_epochs: Option<u8>) {
        self.tx_max_expiry_epochs = tx_max_expiry_epochs;
    }

    pub fn set_external_committees(&mut self, external_committees: Vec<CommitteeIdDef>) {
        self.external_committees = external_committees;
    }

    pub fn external_wallets(&self) -> Vec<ExternalWalletTemplate> {
        self.external_wallets.clone()
    }

    pub fn set_external_wallets(&mut self, external_wallets: Vec<ExternalWalletTemplate>) {
        self.external_wallets = external_wallets;
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

    pub fn set_block_content_max_size(&mut self, block_content_max_size: BlockContentMaxSize) {
        self.block_content_max_size = block_content_max_size;
    }

    pub fn block_content_max_size(&self) -> &BlockContentMaxSize {
        &self.block_content_max_size
    }

    pub fn vote_plan(&self, alias: &str) -> Option<VotePlanDef> {
        self.vote_plans()
            .iter()
            .cloned()
            .find(|x| x.alias() == alias)
    }

    pub fn add_committee<S: Into<NodeAlias>>(&mut self, alias: S) {
        self.committees.push(alias.into());
    }

    pub fn add_external_committee(&mut self, committee: CommitteeIdDef) {
        self.external_committees.push(committee);
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

    pub fn kes_update_speed(&self) -> &KesUpdateSpeed {
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
