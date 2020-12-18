pub mod args;
mod network;

use crate::{
    scenario::{
        ControllerBuilder,TopologyBuilder,NumberOfSlotsPerEpoch,SlotDuration,KESUpdateSpeed,ActiveSlotCoefficient,Milli,ConsensusVersion,
        ContextChaCha,
    },
};

use jormungandr_testing_utils::testing::network_builder::{Blockchain,Node,WalletTemplate};
use chain_impl_mockchain::{
    value::Value,
    testing::scenario::template::{VotePlanDefBuilder,ProposalDefBuilder}
};

pub const LEADER_1: &str = "Leader1";
pub const LEADER_2: &str = "Leader2";
pub const LEADER_3: &str = "Leader3";
pub const LEADER_4: &str = "Leader4";
pub const WALLET_NODE: &str = "Wallet_Node";

pub struct QuickVitBackendSettingsBuilder{
    initials: Vec<u64>,
    vote_start: u32,
    vote_tally: u32,
    tally_end: u32,
    proposals: u32,
    slot_duration: u8,
    slots_per_epoch: u32,
    voting_power: u64
}

impl Default for QuickVitBackendSettingsBuilder{
    fn default() -> Self {
        Self::new()
    }
}


impl QuickVitBackendSettingsBuilder {

    pub fn new() -> Self {
        let initials: Vec<u64> = std::iter::from_fn(|| Some(10_000)).take(10).collect();
        QuickVitBackendSettingsBuilder{
            initials: initials,
            vote_start: 1,
            vote_tally : 2,
            tally_end: 3,
            proposals:  100,
            slot_duration: 20,
            slots_per_epoch: 30,
            voting_power: 8000_000000
        }
    }

    pub fn initials(&mut self, initials: Vec<u64>) -> &mut Self {
        self.initials = initials;
        self
    }
    pub fn initials_count(&mut self, initials_count: usize) -> &mut Self {
        let initials: Vec<u64> = std::iter::from_fn(|| Some(10_000)).take(initials_count).collect();
        self.initials(initials);
        self
    }
    pub fn vote_start_epoch(&mut self, vote_start_epoch: u32) -> &mut Self {
        self.vote_start= vote_start_epoch;
        self
    }
    pub fn tally_start_epoch(&mut self, tally_start_epoch: u32) -> &mut Self {
        self.vote_tally = tally_start_epoch;
        self        
    }
    pub fn tally_end_epoch(&mut self, tally_end_epoch: u32) -> &mut Self {
        self.tally_end = tally_end_epoch;
        self
    }
    pub fn slot_duration(&mut self, slot_duration: u8) -> &mut Self {
        self.slot_duration = slot_duration;
        self        
    }
    pub fn slots_in_epoch(&mut self, slots_in_epoch: u32) -> &mut Self {
        self.slots_per_epoch = slots_in_epoch;
        self
    }
    pub fn proposals_count(&mut self, proposals_count: u32) -> &mut Self {
        self.proposals = proposals_count;
        self        
    }
    pub fn voting_power(&mut self, voting_power: u64) -> &mut Self {
        self.voting_power = voting_power;
        self
    }

    pub fn build_settings(self, context: &mut ContextChaCha) -> ControllerBuilder {
        let committe_wallet_name = "committee";
    
        let mut builder = ControllerBuilder::new("vit_backend");
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
                NumberOfSlotsPerEpoch::new(self.slots_per_epoch).expect("valid number of slots per epoch"),
                SlotDuration::new(self.slot_duration).expect("valid slot duration in seconds"),
                KESUpdateSpeed::new(46800).expect("valid kes update speed in seconds"),
                ActiveSlotCoefficient::new(Milli::from_millis(700)).expect("active slot coefficient in millis"),
            );
    
            blockchain.add_leader(LEADER_1);
            blockchain.add_leader(LEADER_2);
            blockchain.add_leader(LEADER_3);
            blockchain.add_leader(LEADER_4);
    
    
            let committe_wallet = WalletTemplate::new_account(
                committe_wallet_name,
                Value(1_000_000).into()
            );
            blockchain.add_wallet(committe_wallet);
            let mut i = 1;
            for initial in self.initials {
                    
                let wallet = WalletTemplate::new_utxo(
                        format!("wallet_{}_with_{}",i,initial),
                        Value(initial).into());
                blockchain.add_wallet(wallet);
                i = i + 1;
            }
           
            blockchain.add_committee(committe_wallet_name);
           
            let mut vote_plan_builder = VotePlanDefBuilder::new("fund_3");
            vote_plan_builder.owner(committe_wallet_name);
            vote_plan_builder.vote_phases(self.vote_start,self.vote_tally,self.tally_end);
    
    
            for _ in 0..self.proposals {
                let mut proposal_builder = ProposalDefBuilder::new(chain_impl_mockchain::testing::VoteTestGen::external_proposal_id());
                proposal_builder.options(3);
                proposal_builder.action_off_chain();
                vote_plan_builder.with_proposal(&mut proposal_builder);
            }
            blockchain.add_vote_plan(vote_plan_builder.build());
            builder.set_blockchain(blockchain);
            builder.build_settings(context);
            builder
    }
}