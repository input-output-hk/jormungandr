use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{Proposal, Proposals, VoteAction, VotePlan},
    testing::VoteTestGen,
    tokens::identifier::TokenIdentifier,
    vote::{Options, PayloadType},
};
use chain_vote::MemberPublicKey;
use std::str::FromStr;
pub struct VotePlanBuilder {
    proposals_count: usize,
    action: VoteAction,
    payload: PayloadType,
    member_keys: Vec<MemberPublicKey>,
    vote_start: BlockDate,
    tally_start: BlockDate,
    tally_end: BlockDate,
    voting_token: TokenIdentifier,
    options_size: u8,
}

impl Default for VotePlanBuilder {
    fn default() -> Self {
        VotePlanBuilder::new()
    }
}

impl VotePlanBuilder {
    pub fn new() -> Self {
        Self {
            proposals_count: 3,
            options_size: 3,
            action: VoteAction::OffChain,
            payload: PayloadType::Public,
            member_keys: Vec::new(),
            vote_start: BlockDate::from_epoch_slot_id(0, 0),
            tally_start: BlockDate::from_epoch_slot_id(1, 0),
            tally_end: BlockDate::from_epoch_slot_id(2, 0),
            voting_token: TokenIdentifier::from_str(
                "00000000000000000000000000000000000000000000000000000000.00000000",
            )
            .unwrap(),
        }
    }

    pub fn proposals_count(&mut self, proposals_count: usize) -> &mut Self {
        self.proposals_count = proposals_count;
        self
    }

    pub fn action_type(&mut self, action: VoteAction) -> &mut Self {
        self.action = action;
        self
    }

    pub fn private(&mut self) -> &mut Self {
        self.payload = PayloadType::Private;
        self
    }

    pub fn public(&mut self) -> &mut Self {
        self.payload = PayloadType::Public;
        self
    }

    pub fn member_public_key(&mut self, key: MemberPublicKey) -> &mut Self {
        self.member_keys.push(key);
        self
    }

    pub fn member_public_keys(&mut self, keys: Vec<MemberPublicKey>) -> &mut Self {
        for key in keys {
            self.member_public_key(key);
        }
        self
    }

    pub fn vote_start(&mut self, block_date: BlockDate) -> &mut Self {
        self.vote_start = block_date;
        self
    }

    pub fn tally_start(&mut self, block_date: BlockDate) -> &mut Self {
        self.tally_start = block_date;
        self
    }

    pub fn tally_end(&mut self, block_date: BlockDate) -> &mut Self {
        self.tally_end = block_date;
        self
    }

    pub fn voting_token(&mut self, voting_token: TokenIdentifier) -> &mut Self {
        self.voting_token = voting_token;
        self
    }

    pub fn options_size(&mut self, size: u8) -> &mut Self {
        self.options_size = size;
        self
    }

    pub fn build(&self) -> VotePlan {
        let proposal_vec: Vec<Proposal> = std::iter::from_fn(|| {
            Some(Proposal::new(
                VoteTestGen::external_proposal_id(),
                Options::new_length(self.options_size).unwrap(),
                self.action.clone(),
            ))
        })
        .take(self.proposals_count)
        .collect();

        let mut proposals = Proposals::new();
        for proposal in proposal_vec {
            let _ = proposals.push(proposal);
        }

        VotePlan::new(
            self.vote_start,
            self.tally_start,
            self.tally_end,
            proposals,
            self.payload,
            self.member_keys.clone(),
            self.voting_token.clone(),
        )
    }
}
