pub mod args;
mod network;

use crate::scenario::{
    ActiveSlotCoefficient, ConsensusVersion, ContextChaCha, Controller, ControllerBuilder,
    KESUpdateSpeed, Milli, NumberOfSlotsPerEpoch, SlotDuration, TopologyBuilder,
};
use crate::test::Result;
use assert_fs::fixture::PathChild;
use chain_crypto::SecretKey;
use chain_impl_mockchain::testing::scenario::template::VotePlanDef;
use chain_impl_mockchain::vote::PayloadType;
use chain_impl_mockchain::{
    testing::scenario::template::{ProposalDefBuilder, VotePlanDefBuilder},
    value::Value,
};
use chrono::naive::NaiveDateTime;
use jormungandr_lib::time::SecondsSinceUnixEpoch;
use jormungandr_testing_utils::qr_code::KeyQrCode;
use jormungandr_testing_utils::testing::network_builder::{Blockchain, Node, WalletTemplate};
use vit_servicing_station_tests::common::data::ValidVotePlanParameters;

pub const LEADER_1: &str = "Leader1";
pub const LEADER_2: &str = "Leader2";
pub const LEADER_3: &str = "Leader3";
pub const LEADER_4: &str = "Leader4";
pub const WALLET_NODE: &str = "Wallet_Node";

pub struct QuickVitBackendSettingsBuilder {
    initials: Vec<u64>,
    vote_start: u64,
    vote_tally: u64,
    tally_end: u64,
    vote_start_timestamp: Option<NaiveDateTime>,
    tally_start_timestamp: Option<NaiveDateTime>,
    tally_end_timestamp: Option<NaiveDateTime>,
    next_vote_start_time: Option<NaiveDateTime>,
    proposals: u32,
    slot_duration: u8,
    slots_per_epoch: u32,
    voting_power: u64,
    fund_name: String,
    private: bool,
}

impl Default for QuickVitBackendSettingsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

const FORMAT: &str = "%Y-%m-%d %H:%M:%S";

impl QuickVitBackendSettingsBuilder {
    pub fn new() -> Self {
        let initials: Vec<u64> = std::iter::from_fn(|| Some(10_000)).take(10).collect();

        QuickVitBackendSettingsBuilder {
            initials,
            vote_start: 1,
            vote_tally: 2,
            tally_end: 3,
            proposals: 100,
            slot_duration: 20,
            slots_per_epoch: 30,
            voting_power: 8000_000000,
            vote_start_timestamp: None,
            tally_start_timestamp: None,
            tally_end_timestamp: None,
            next_vote_start_time: None,
            fund_name: "fund_3".to_owned(),
            private: false,
        }
    }

    pub fn initials(&mut self, initials: Vec<u64>) -> &mut Self {
        self.initials = initials;
        self
    }
    pub fn initials_count(&mut self, initials_count: usize) -> &mut Self {
        let initials: Vec<u64> = std::iter::from_fn(|| Some(10_000))
            .take(initials_count)
            .collect();
        self.initials(initials);
        self
    }
    pub fn vote_start_epoch(&mut self, vote_start_epoch: u32) -> &mut Self {
        self.vote_start = vote_start_epoch as u64;
        self
    }

    pub fn tally_start_epoch(&mut self, tally_start_epoch: u32) -> &mut Self {
        self.vote_tally = tally_start_epoch as u64;
        self
    }
    pub fn tally_end_epoch(&mut self, tally_end_epoch: u32) -> &mut Self {
        self.tally_end = tally_end_epoch as u64;
        self
    }

    pub fn slot_duration_in_seconds(&mut self, slot_duration: u8) -> &mut Self {
        self.slot_duration = slot_duration;
        self
    }
    pub fn slots_in_epoch_count(&mut self, slots_in_epoch: u32) -> &mut Self {
        self.slots_per_epoch = slots_in_epoch;
        self
    }
    pub fn proposals_count(&mut self, proposals_count: u32) -> &mut Self {
        self.proposals = proposals_count;
        self
    }
    pub fn voting_power(&mut self, voting_power: u64) -> &mut Self {
        self.voting_power = voting_power * 1_000_000;
        self
    }

    pub fn next_vote_timestamp(&mut self, next_vote_timestamp: Option<String>) -> &mut Self {
        if let Some(timestamp) = next_vote_timestamp {
            self.next_vote_start_time =
                Some(NaiveDateTime::parse_from_str(&timestamp, FORMAT).unwrap());
        }
        self
    }

    pub fn vote_start_timestamp(&mut self, vote_start_timestamp: Option<String>) -> &mut Self {
        if let Some(timestamp) = vote_start_timestamp {
            self.vote_start_timestamp =
                Some(NaiveDateTime::parse_from_str(&timestamp, FORMAT).unwrap());
        }
        self
    }

    pub fn tally_start_timestamp(&mut self, tally_start_timestamp: Option<String>) -> &mut Self {
        if let Some(timestamp) = tally_start_timestamp {
            self.tally_start_timestamp =
                Some(NaiveDateTime::parse_from_str(&timestamp, FORMAT).unwrap());
        }
        self
    }

    pub fn tally_end_timestamp(&mut self, tally_end_timestamp: Option<String>) -> &mut Self {
        if let Some(timestamp) = tally_end_timestamp {
            self.tally_end_timestamp =
                Some(NaiveDateTime::parse_from_str(&timestamp, FORMAT).unwrap());
        }
        self
    }

    pub fn fund_name(&self) -> String {
        self.fund_name.to_string()
    }

    pub fn private(&mut self, private: bool) {
        self.private = private;
    }

    pub fn recalculate_voting_periods_if_needed(&mut self, block0_date: SecondsSinceUnixEpoch) {
        let epoch_duration: u64 = self.slot_duration as u64 * self.slots_per_epoch as u64;
        if self.vote_start_timestamp.is_none() {
            println!(
                "{:?}",
                NaiveDateTime::from_timestamp(block0_date.to_secs() as i64, 0)
            );
            let vote_start_timestamp = block0_date.to_secs() + epoch_duration * self.vote_start;
            self.vote_start_timestamp = Some(NaiveDateTime::from_timestamp(
                vote_start_timestamp as i64,
                0,
            ));
            let tally_start_timestamp = block0_date.to_secs() + epoch_duration * self.vote_tally;
            self.tally_start_timestamp = Some(NaiveDateTime::from_timestamp(
                tally_start_timestamp as i64,
                0,
            ));
            let tally_end_timestamp = block0_date.to_secs() + epoch_duration * self.tally_end;
            self.tally_end_timestamp =
                Some(NaiveDateTime::from_timestamp(tally_end_timestamp as i64, 0));
        }

        if self.next_vote_start_time.is_none() {
            let timestamp =
                SecondsSinceUnixEpoch::now().to_secs() + epoch_duration * self.tally_end + 10;
            self.next_vote_start_time = Some(NaiveDateTime::from_timestamp(timestamp as i64, 0));
        }
    }

    pub fn parameters(&self, vote_plan: VotePlanDef) -> ValidVotePlanParameters {
        let mut parameters = ValidVotePlanParameters::new(vote_plan);
        parameters.set_voting_power_threshold(self.voting_power as i64);
        parameters.set_voting_start(self.vote_start_timestamp.unwrap().timestamp());
        parameters.set_voting_tally_start(self.tally_start_timestamp.unwrap().timestamp());
        parameters.set_voting_tally_end(self.tally_end_timestamp.unwrap().timestamp());
        parameters.set_next_fund_start_time(self.next_vote_start_time.unwrap().timestamp());
        parameters
    }

    pub fn build(
        &mut self,
        mut context: ContextChaCha,
    ) -> Result<(Controller, ValidVotePlanParameters)> {
        let committe_wallet_name = "committee";
        let title = "vit_backend";

        let mut builder = ControllerBuilder::new(title);
        let mut topology_builder = TopologyBuilder::new();

        // Leader 1
        let leader_1 = Node::new(LEADER_1);
        topology_builder.register_node(leader_1);

        // leader 2
        let mut leader_2 = Node::new(LEADER_2);
        leader_2.add_trusted_peer(LEADER_1);
        topology_builder.register_node(leader_2);

        // leader 3
        let mut leader_3 = Node::new(LEADER_3);
        leader_3.add_trusted_peer(LEADER_1);
        leader_3.add_trusted_peer(LEADER_2);
        topology_builder.register_node(leader_3);

        // leader 4
        let mut leader_4 = Node::new(LEADER_4);
        leader_4.add_trusted_peer(LEADER_1);
        leader_4.add_trusted_peer(LEADER_2);
        leader_4.add_trusted_peer(LEADER_3);
        topology_builder.register_node(leader_4);

        // passive
        let mut passive = Node::new(WALLET_NODE);
        passive.add_trusted_peer(LEADER_1);
        passive.add_trusted_peer(LEADER_2);
        passive.add_trusted_peer(LEADER_3);
        passive.add_trusted_peer(LEADER_4);

        topology_builder.register_node(passive);

        builder.set_topology(topology_builder.build());

        let mut blockchain = Blockchain::new(
            ConsensusVersion::Bft,
            NumberOfSlotsPerEpoch::new(self.slots_per_epoch)
                .expect("valid number of slots per epoch"),
            SlotDuration::new(self.slot_duration).expect("valid slot duration in seconds"),
            KESUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
            ActiveSlotCoefficient::new(Milli::from_millis(700))
                .expect("active slot coefficient in millis"),
        );

        blockchain.add_leader(LEADER_1);
        blockchain.add_leader(LEADER_2);
        blockchain.add_leader(LEADER_3);
        blockchain.add_leader(LEADER_4);

        let committe_wallet = WalletTemplate::new_account(committe_wallet_name, Value(1_000_000));
        blockchain.add_wallet(committe_wallet);
        let mut i = 1u32;

        let child = context.child_directory(title);

        for initial in self.initials.iter() {
            let wallet_alias = format!("wallet_{}_with_{}", i, initial);
            let wallet = WalletTemplate::new_utxo(wallet_alias.clone(), Value(*initial));

            let password = "1234".to_owned();

            let sk = SecretKey::generate(rand::thread_rng());
            let qr = KeyQrCode::generate(sk, &password.as_bytes().to_vec());
            let svg = child.child(format!("{}_{}.svg", wallet_alias, password));
            qr.write_svg(svg.path()).unwrap();

            blockchain.add_wallet(wallet);
            i += 1;
        }

        blockchain.add_committee(committe_wallet_name);

        let mut vote_plan_builder = VotePlanDefBuilder::new(&self.fund_name());
        vote_plan_builder.owner(committe_wallet_name);

        if self.private {
            vote_plan_builder.payload_type(PayloadType::Private);
        }
        vote_plan_builder.vote_phases(
            self.vote_start as u32,
            self.vote_tally as u32,
            self.tally_end as u32,
        );

        for _ in 0..self.proposals {
            let mut proposal_builder = ProposalDefBuilder::new(
                chain_impl_mockchain::testing::VoteTestGen::external_proposal_id(),
            );
            proposal_builder.options(3);

            proposal_builder.action_off_chain();
            vote_plan_builder.with_proposal(&mut proposal_builder);
        }

        let vote_plan = vote_plan_builder.build();
        blockchain.add_vote_plan(vote_plan.clone());
        builder.set_blockchain(blockchain);
        builder.build_settings(&mut context);

        let controller = builder.build(context)?;

        controller.settings().dump_private_vote_keys(child);

        self.recalculate_voting_periods_if_needed(
            controller
                .settings()
                .network_settings
                .block0
                .blockchain_configuration
                .block0_date,
        );
        Ok((controller, self.parameters(vote_plan)))
    }
}
