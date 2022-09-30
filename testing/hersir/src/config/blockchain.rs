use super::NodeAlias;
use crate::builder::VotePlanKey;
use chain_addr::Discrimination;
pub use chain_impl_mockchain::chaintypes::ConsensusVersion;
use chain_impl_mockchain::{fee::LinearFee, milli::Milli};
use jormungandr_lib::{
    interfaces::{
        ActiveSlotCoefficient, BlockContentMaxSize, CommitteeIdDef, ConsensusLeaderId,
        ConsensusVersionDef, DiscriminationDef, KesUpdateSpeed, LinearFeeDef,
        NumberOfSlotsPerEpoch, SlotDuration, VotePlan,
    },
    time::SecondsSinceUnixEpoch,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Blockchain {
    #[serde(default)]
    block0_date: SecondsSinceUnixEpoch,
    #[serde(default)]
    block_content_max_size: BlockContentMaxSize,
    #[serde(with = "ConsensusVersionDef")]
    consensus: ConsensusVersion,
    #[serde(default)]
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    #[serde(with = "DiscriminationDef")]
    discrimination: Discrimination,
    #[serde(default)]
    committees: Vec<CommitteeIdDef>,
    #[serde(default)]
    external_consensus_leader_ids: Vec<ConsensusLeaderId>,
    #[serde(default)]
    kes_update_speed: KesUpdateSpeed,
    #[serde(default)]
    leaders: Vec<NodeAlias>,
    #[serde(with = "LinearFeeDef")]
    linear_fee: LinearFee,
    #[serde(default)]
    slot_duration: SlotDuration,
    #[serde(default)]
    slots_per_epoch: NumberOfSlotsPerEpoch,
    #[serde(default)]
    tx_max_expiry_epochs: Option<u8>,
    #[serde(default)]
    vote_plans: HashMap<VotePlanKey, VotePlan>,
}

impl Blockchain {
    pub fn block0_date(&self) -> SecondsSinceUnixEpoch {
        self.block0_date
    }

    pub fn with_block0_date(mut self, block0_date: SecondsSinceUnixEpoch) -> Self {
        self.block0_date = block0_date;
        self
    }

    pub fn block_content_max_size(&self) -> &BlockContentMaxSize {
        &self.block_content_max_size
    }

    pub fn with_block_content_max_size(
        mut self,
        block_content_max_size: BlockContentMaxSize,
    ) -> Self {
        self.block_content_max_size = block_content_max_size;
        self
    }

    pub fn consensus(&self) -> &ConsensusVersion {
        &self.consensus
    }

    pub fn with_consensus(mut self, consensus: ConsensusVersion) -> Self {
        self.consensus = consensus;
        self
    }

    pub fn consensus_genesis_praos_active_slot_coeff(&self) -> &ActiveSlotCoefficient {
        &self.consensus_genesis_praos_active_slot_coeff
    }

    pub fn with_consensus_genesis_praos_active_slot_coeff(
        mut self,
        coeff: ActiveSlotCoefficient,
    ) -> Self {
        self.consensus_genesis_praos_active_slot_coeff = coeff;
        self
    }

    pub fn discrimination(&self) -> Discrimination {
        self.discrimination
    }

    pub fn with_discrimination(mut self, discrimination: Discrimination) -> Self {
        self.discrimination = discrimination;
        self
    }

    pub fn committees(&self) -> Vec<CommitteeIdDef> {
        self.committees.clone()
    }

    pub fn with_committees(mut self, committees: Vec<CommitteeIdDef>) -> Self {
        self.committees = committees;
        self
    }

    pub fn with_committee(mut self, committee: CommitteeIdDef) -> Self {
        self.committees.push(committee);
        self
    }

    pub fn external_consensus_leader_ids(&self) -> Vec<ConsensusLeaderId> {
        self.external_consensus_leader_ids.clone()
    }

    pub fn with_external_consensus_leader_ids(
        mut self,
        external_consensus_leader_ids: Vec<ConsensusLeaderId>,
    ) -> Self {
        self.external_consensus_leader_ids = external_consensus_leader_ids;
        self
    }

    pub fn has_external_consensus_leader_ids(&self) -> bool {
        !self.external_consensus_leader_ids().is_empty()
    }

    pub fn kes_update_speed(&self) -> &KesUpdateSpeed {
        &self.kes_update_speed
    }

    pub fn leaders(&self) -> impl Iterator<Item = &NodeAlias> {
        self.leaders.iter()
    }

    pub fn with_leader<S: Into<NodeAlias>>(mut self, alias: S) -> Self {
        self.leaders.push(alias.into());
        self
    }

    pub fn with_leaders<S: Into<NodeAlias>>(mut self, aliases: Vec<S>) -> Self {
        self.leaders.extend(aliases.into_iter().map(Into::into));
        self
    }

    pub fn linear_fee(&self) -> LinearFee {
        self.linear_fee.clone()
    }

    pub fn with_linear_fee(mut self, linear_fee: LinearFee) -> Self {
        self.linear_fee = linear_fee;
        self
    }

    pub fn slot_duration(&self) -> &SlotDuration {
        &self.slot_duration
    }

    pub fn with_slot_duration(mut self, slot_duration: SlotDuration) -> Self {
        self.slot_duration = slot_duration;
        self
    }

    pub fn slots_per_epoch(&self) -> &NumberOfSlotsPerEpoch {
        &self.slots_per_epoch
    }

    pub fn with_slots_per_epoch(mut self, slots_per_epoch: NumberOfSlotsPerEpoch) -> Self {
        self.slots_per_epoch = slots_per_epoch;
        self
    }

    pub fn tx_max_expiry_epochs(&self) -> Option<u8> {
        self.tx_max_expiry_epochs
    }

    pub fn with_tx_max_expiry_epochs(mut self, tx_max_expiry_epochs: Option<u8>) -> Self {
        self.tx_max_expiry_epochs = tx_max_expiry_epochs;
        self
    }

    pub fn vote_plans(&self) -> HashMap<VotePlanKey, VotePlan> {
        self.vote_plans.clone()
    }

    pub fn with_vote_plan(
        mut self,
        alias: String,
        owner_alias: String,
        vote_plan_template: VotePlan,
    ) -> Self {
        self.vote_plans
            .insert(VotePlanKey { alias, owner_alias }, vote_plan_template);
        self
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self {
            block0_date: Default::default(),
            block_content_max_size: BlockContentMaxSize::default(),
            committees: Vec::new(),
            consensus: ConsensusVersion::GenesisPraos,
            consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::new(
                Milli::from_millis(500),
            )
            .unwrap(),
            discrimination: Discrimination::Test,
            external_consensus_leader_ids: Vec::new(),
            kes_update_speed: KesUpdateSpeed::new(46800).unwrap(),
            leaders: Vec::new(),
            linear_fee: LinearFee::new(1, 1, 1),
            slot_duration: SlotDuration::new(2).unwrap(),
            slots_per_epoch: NumberOfSlotsPerEpoch::new(60).unwrap(),
            tx_max_expiry_epochs: None,
            vote_plans: HashMap::new(),
        }
    }
}

#[derive(Default)]
pub struct BlockchainBuilder {
    blockchain: Blockchain,
}

impl BlockchainBuilder {
    pub fn block0_date(mut self, block0_date: SecondsSinceUnixEpoch) -> Self {
        self.blockchain = self.blockchain.with_block0_date(block0_date);
        self
    }

    pub fn block_content_max_size(mut self, block_content_max_size: BlockContentMaxSize) -> Self {
        self.blockchain = self
            .blockchain
            .with_block_content_max_size(block_content_max_size);
        self
    }

    pub fn committee(mut self, committee_id: CommitteeIdDef) -> Self {
        self.blockchain = self.blockchain.with_committee(committee_id);
        self
    }

    pub fn consensus(mut self, consensus: ConsensusVersion) -> Self {
        self.blockchain = self.blockchain.with_consensus(consensus);
        self
    }

    pub fn consensus_genesis_praos_active_slot_coeff(
        mut self,
        coeff: ActiveSlotCoefficient,
    ) -> Self {
        self.blockchain = self
            .blockchain
            .with_consensus_genesis_praos_active_slot_coeff(coeff);
        self
    }

    pub fn discrimination(mut self, discrimination: Discrimination) -> Self {
        self.blockchain = self.blockchain.with_discrimination(discrimination);
        self
    }

    pub fn committees(mut self, committees: Vec<CommitteeIdDef>) -> Self {
        self.blockchain = self.blockchain.with_committees(committees);
        self
    }

    pub fn external_consensus_leader_ids(
        mut self,
        external_consensus_leader_ids: Vec<ConsensusLeaderId>,
    ) -> Self {
        self.blockchain = self
            .blockchain
            .with_external_consensus_leader_ids(external_consensus_leader_ids);
        self
    }

    pub fn leader<S: Into<NodeAlias>>(mut self, alias: S) -> Self {
        self.blockchain = self.blockchain.with_leader(alias.into());
        self
    }

    pub fn leaders<S: Into<NodeAlias>>(mut self, aliases: Vec<S>) -> Self {
        self.blockchain = self.blockchain.with_leaders(aliases);
        self
    }
    pub fn linear_fee(mut self, linear_fee: LinearFee) -> Self {
        self.blockchain = self.blockchain.with_linear_fee(linear_fee);
        self
    }

    pub fn slot_duration(mut self, slot_duration: u8) -> Self {
        self.blockchain = self
            .blockchain
            .with_slot_duration(SlotDuration::new(slot_duration).unwrap());
        self
    }

    pub fn slots_per_epoch(mut self, slots_per_epoch: u32) -> Self {
        self.blockchain = self
            .blockchain
            .with_slots_per_epoch(NumberOfSlotsPerEpoch::new(slots_per_epoch).unwrap());
        self
    }

    pub fn tx_max_expiry_epochs(mut self, tx_max_expiry_epochs: Option<u8>) -> Self {
        self.blockchain = self
            .blockchain
            .with_tx_max_expiry_epochs(tx_max_expiry_epochs);
        self
    }

    pub fn vote_plan(
        mut self,
        alias: String,
        owner_alias: String,
        vote_plan_template: VotePlan,
    ) -> Self {
        self.blockchain = self
            .blockchain
            .with_vote_plan(alias, owner_alias, vote_plan_template);
        self
    }

    pub fn build(self) -> Blockchain {
        self.blockchain
    }
}
