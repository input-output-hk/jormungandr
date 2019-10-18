use crate::{
    config::ConfigParam,
    fragment::config::ConfigParams,
    leadership::bft::LeaderId,
    update::{
        SignedUpdateProposal, SignedUpdateVote, UpdateProposal, UpdateProposalId,
        UpdateProposalWithProposer, UpdateVote,
    },
};

pub struct ProposalBuilder {
    config_params: ConfigParams,
}

impl ProposalBuilder {
    pub fn new() -> Self {
        ProposalBuilder {
            config_params: ConfigParams::new(),
        }
    }
    pub fn with_proposal_changes(&mut self, changes: Vec<ConfigParam>) -> &mut Self {
        for change in changes {
            self.with_proposal_change(change);
        }
        self
    }

    pub fn with_proposal_change(&mut self, change: ConfigParam) -> &mut Self {
        self.config_params.push(change);
        self
    }

    pub fn build(&self) -> UpdateProposal {
        let mut update_proposal = UpdateProposal::new();
        for config_param in self.config_params.iter().cloned() {
            update_proposal.changes.push(config_param);
        }
        update_proposal
    }
}

pub struct SignedProposalBuilder {
    update_proposal: Option<UpdateProposal>,
    proposer_id: Option<LeaderId>,
}

impl SignedProposalBuilder {
    pub fn new() -> Self {
        SignedProposalBuilder {
            update_proposal: None,
            proposer_id: None,
        }
    }

    pub fn with_proposer_id(&mut self, proposer_id: LeaderId) -> &mut Self {
        self.proposer_id = Some(proposer_id);
        self
    }

    pub fn with_proposal_update(&mut self, update_proposal: UpdateProposal) -> &mut Self {
        self.update_proposal = Some(update_proposal);
        self
    }

    pub fn build(&self) -> SignedUpdateProposal {
        SignedUpdateProposal {
            proposal: UpdateProposalWithProposer {
                proposal: self.update_proposal.clone().unwrap(),
                proposer_id: self.proposer_id.clone().unwrap(),
            },
        }
    }
}

pub struct UpdateVoteBuilder {
    proposal_id: Option<UpdateProposalId>,
    voter_id: Option<LeaderId>,
}

impl UpdateVoteBuilder {
    pub fn new() -> Self {
        UpdateVoteBuilder {
            proposal_id: None,
            voter_id: None,
        }
    }

    pub fn with_proposal_id(&mut self, proposal_id: UpdateProposalId) -> &mut Self {
        self.proposal_id = Some(proposal_id);
        self
    }

    pub fn with_voter_id(&mut self, voter_id: LeaderId) -> &mut Self {
        self.voter_id = Some(voter_id);
        self
    }

    pub fn build(&self) -> SignedUpdateVote {
        let update_vote = UpdateVote {
            proposal_id: self.proposal_id.unwrap().clone(),
            voter_id: self.voter_id.clone().unwrap(),
        };
        SignedUpdateVote { vote: update_vote }
    }
}
