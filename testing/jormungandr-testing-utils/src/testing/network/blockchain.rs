use super::{ExternalWalletTemplate, NodeAlias, WalletAlias, WalletTemplate};
use crate::testing::network::VotePlanKey;
use chain_addr::Discrimination;
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::fee::LinearFee;
use chain_impl_mockchain::milli::Milli;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
    ConsensusVersionDef, DiscriminationDef, KesUpdateSpeed, LinearFeeDef, NumberOfSlotsPerEpoch,
    SlotDuration, VotePlan,
};
use jormungandr_lib::time::SecondsSinceUnixEpoch;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Blockchain {
    #[serde(default)]
    block_content_max_size: BlockContentMaxSize,
    #[serde(default)]
    committees: Vec<WalletAlias>,
    #[serde(with = "ConsensusVersionDef")]
    consensus: ConsensusVersion,
    #[serde(default)]
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    #[serde(with = "DiscriminationDef")]
    discrimination: Discrimination,
    #[serde(default)]
    external_committees: Vec<CommitteeIdDef>,
    #[serde(default)]
    external_wallets: Vec<ExternalWalletTemplate>,
    #[serde(default)]
    kes_update_speed: KesUpdateSpeed,
    block0_date: SecondsSinceUnixEpoch,
    external_consensus_leader_ids: Vec<ConsensusLeaderId>,
    #[serde(default)]
    leaders: Vec<NodeAlias>,
    #[serde(with = "LinearFeeDef")]
    linear_fee: LinearFee,
    #[serde(default)]
    slot_duration: SlotDuration,
    #[serde(default)]
    slots_per_epoch: NumberOfSlotsPerEpoch,
    #[serde(default)]
    vote_plans: HashMap<VotePlanKey, VotePlan>,
    #[serde(default)]
    wallets: HashMap<WalletAlias, WalletTemplate>,
    #[serde(default)]
    tx_max_expiry_epochs: Option<u8>,
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
            block_content_max_size: BlockContentMaxSize::default(),
            committees: Vec::new(),
            consensus,
            consensus_genesis_praos_active_slot_coeff,
            discrimination: Discrimination::Test,
            external_committees: Vec::new(),
            external_wallets: Vec::new(),
            block0_date: Default::default(),
            external_consensus_leader_ids: Vec::new(),
            slots_per_epoch,
            slot_duration,
            kes_update_speed,
            leaders: Vec::new(),
            linear_fee: LinearFee::new(1, 1, 1),
            vote_plans: HashMap::new(),
            wallets: HashMap::new(),
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

    pub fn set_block0_date(&mut self, block0_date: SecondsSinceUnixEpoch) {
        self.block0_date = block0_date;
    }

    pub fn block0_date(&self) -> SecondsSinceUnixEpoch {
        self.block0_date
    }

    pub fn set_external_committees(&mut self, external_committees: Vec<CommitteeIdDef>) {
        self.external_committees = external_committees;
    }

    pub fn set_external_consensus_leader_ids(
        &mut self,
        external_consensus_leader_ids: Vec<ConsensusLeaderId>,
    ) {
        self.external_consensus_leader_ids = external_consensus_leader_ids;
    }

    pub fn has_external_consensus_leader_ids(&self) -> bool {
        !self.external_consensus_leader_ids().is_empty()
    }

    pub fn external_consensus_leader_ids(&self) -> Vec<ConsensusLeaderId> {
        self.external_consensus_leader_ids.clone()
    }

    pub fn set_external_wallets(&mut self, external_wallets: Vec<ExternalWalletTemplate>) {
        self.external_wallets = external_wallets;
    }

    pub fn external_wallets(&self) -> Vec<ExternalWalletTemplate> {
        self.external_wallets.clone()
    }

    pub fn vote_plans(&self) -> HashMap<VotePlanKey, VotePlan> {
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

    pub fn add_committee<S: Into<NodeAlias>>(&mut self, alias: S) {
        self.committees.push(alias.into());
    }

    pub fn add_external_committee(&mut self, committee: CommitteeIdDef) {
        self.external_committees.push(committee);
    }

    pub fn add_vote_plan(
        &mut self,
        alias: String,
        owner_alias: String,
        vote_plan_template: VotePlan,
    ) {
        self.vote_plans
            .insert(VotePlanKey { alias, owner_alias }, vote_plan_template);
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

impl Default for Blockchain {
    fn default() -> Self {
        Self::new(
            ConsensusVersion::GenesisPraos,
            NumberOfSlotsPerEpoch::new(60).expect("valid number of slots per epoch"),
            SlotDuration::new(2).expect("valid slot duration in seconds"),
            KesUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
            ActiveSlotCoefficient::new(Milli::from_millis(999))
                .expect("active slot coefficient in millis"),
        )
    }
}
