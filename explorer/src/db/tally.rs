use super::indexing::ExplorerVoteProposal;
use crate::db::indexing::{ExplorerVote, ExplorerVoteTally};
use chain_impl_mockchain::{certificate::DecryptedPrivateTallyProposal, stake::StakeControl};

pub fn compute_private_tally(
    proposal: &ExplorerVoteProposal,
    tally: &DecryptedPrivateTallyProposal,
) -> ExplorerVoteTally {
    ExplorerVoteTally::Private {
        results: Some(tally.tally_result.iter().cloned().map(Into::into).collect()),
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
        results: results.into_iter().map(u64::into).collect(),
        options: proposal.options.clone(),
    }
}
