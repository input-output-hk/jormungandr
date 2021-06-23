use chain_impl_mockchain::stake::StakeControl;

use super::indexing::ExplorerVoteProposal;
use crate::db::indexing::{ExplorerVote, ExplorerVoteTally};

pub fn compute_private_tally(proposal: &ExplorerVoteProposal) -> ExplorerVoteTally {
    tracing::warn!("private tally triggered but it is not implemented");

    ExplorerVoteTally::Private {
        results: None,
        options: proposal.options.clone(),
    }
}

pub fn compute_public_tally(
    proposal: &ExplorerVoteProposal,
    stake: &StakeControl,
) -> ExplorerVoteTally {
    let mut results = vec![0u64; proposal.options.choice_range().end as usize];

    for (address, vote) in proposal.votes.iter() {
        if let Some(account_id) = address.to_single_account() {
            if let Some(stake) = stake.by(&account_id) {
                match vote.as_ref() {
                    ExplorerVote::Public(choice) => {
                        let index = choice.as_byte() as usize;
                        results[index] = results[index].saturating_add(stake.into());
                    }
                    ExplorerVote::Private {
                        proof: _,
                        encrypted_vote: _,
                    } => {
                        unreachable!("internal error: found private vote when computing tally for public proposal")
                    }
                }
            }
        }
    }

    ExplorerVoteTally::Public {
        results: results.drain(..).map(u64::into).collect(),
        options: proposal.options.clone(),
    }
}
