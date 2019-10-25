use crate::{
    config::ConfigParam,
    leadership::bft::LeaderId,
    update::{
        SignedUpdateProposal, SignedUpdateVote, UpdateProposal, UpdateProposalId,
        UpdateProposalWithProposer, UpdateVote,
    },
};

pub fn build_proposal(
    proposer_id: LeaderId,
    config_params: Vec<ConfigParam>,
) -> SignedUpdateProposal {
    //create proposal
    let mut update_proposal = UpdateProposal::new();

    for config_param in config_params {
        update_proposal.changes.push(config_param);
    }

    //add proposer
    let update_proposal_with_proposer = UpdateProposalWithProposer {
        proposal: update_proposal,
        proposer_id: proposer_id.clone(),
    };

    //sign proposal
    SignedUpdateProposal {
        proposal: update_proposal_with_proposer,
    }
}

pub fn build_vote(proposal_id: UpdateProposalId, leader_id: LeaderId) -> SignedUpdateVote {
    let update_vote = UpdateVote {
        proposal_id: proposal_id.clone(),
        voter_id: leader_id.clone(),
    };
    SignedUpdateVote { vote: update_vote }
}
