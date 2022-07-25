use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{ExternalProposalId, Proposal, Proposals, VoteAction, VotePlan},
    testing::VoteTestGen,
    tokens::identifier::TokenIdentifier,
    vote::{Options, PayloadType},
};
use chain_vote::MemberPublicKey;
use std::str::FromStr;
pub struct VotePlanBuilder {
    proposals_builder: ProposalsBuilder,
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
            proposals_builder: ProposalsBuilder::default().with_count(3),
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

    pub fn proposals_count(mut self, proposals_count: usize) -> Self {
        self.proposals_builder = self.proposals_builder.with_count(proposals_count);
        self
    }

    pub fn proposals_external_ids(mut self, proposals_ids: Vec<ExternalProposalId>) -> Self {
        self.proposals_builder = self.proposals_builder.with_ids(proposals_ids);
        self
    }

    pub fn action_type(mut self, action: VoteAction) -> Self {
        self.action = action;
        self
    }

    pub fn private(mut self) -> Self {
        self.payload = PayloadType::Private;
        self
    }

    pub fn public(mut self) -> Self {
        self.payload = PayloadType::Public;
        self
    }

    pub fn member_public_key(mut self, key: MemberPublicKey) -> Self {
        self.member_keys.push(key);
        self
    }

    pub fn member_public_keys(mut self, keys: Vec<MemberPublicKey>) -> Self {
        for key in keys {
            self = self.member_public_key(key);
        }
        self
    }

    pub fn vote_start(mut self, block_date: BlockDate) -> Self {
        self.vote_start = block_date;
        self
    }

    pub fn tally_start(mut self, block_date: BlockDate) -> Self {
        self.tally_start = block_date;
        self
    }

    pub fn tally_end(mut self, block_date: BlockDate) -> Self {
        self.tally_end = block_date;
        self
    }

    pub fn voting_token(mut self, voting_token: TokenIdentifier) -> Self {
        self.voting_token = voting_token;
        self
    }

    pub fn options_size(mut self, size: u8) -> Self {
        self.options_size = size;
        self
    }

    pub fn build(self) -> VotePlan {
        let proposals = self.proposals_builder.build(self.options_size, self.action);

        VotePlan::new(
            self.vote_start,
            self.tally_start,
            self.tally_end,
            proposals,
            self.payload,
            self.member_keys.clone(),
            self.voting_token,
        )
    }
}

#[derive(Debug, Default)]
pub struct ProposalsBuilder {
    count: usize,
    external_ids: Vec<ExternalProposalId>,
}

impl ProposalsBuilder {
    pub fn with_count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    pub fn with_ids(mut self, external_ids: Vec<ExternalProposalId>) -> Self {
        self.external_ids = external_ids;
        self
    }

    pub fn build(self, options_size: u8, action: VoteAction) -> Proposals {
        let proposals_vec: Vec<Proposal> = if !self.external_ids.is_empty() {
            self.external_ids
                .into_iter()
                .map(|epi| {
                    Proposal::new(
                        epi,
                        Options::new_length(options_size).unwrap(),
                        action.clone(),
                    )
                })
                .collect()
        } else {
            std::iter::from_fn(|| {
                Some(Proposal::new(
                    VoteTestGen::external_proposal_id(),
                    Options::new_length(options_size).unwrap(),
                    action.clone(),
                ))
            })
            .take(self.count)
            .collect()
        };
        let mut proposals = Proposals::new();
        for proposal in proposals_vec {
            let _ = proposals.push(proposal);
        }
        proposals
    }
}
