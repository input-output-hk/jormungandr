use crate::key::Hash;
use crate::milli::Milli;
use crate::{
    config::ConfigParam,
    fee::LinearFee,
    fragment::config::ConfigParams,
    leadership::bft::LeaderId,
    testing::builders::SignedProposalBuilder,
    testing::{arbitrary::utils as arbitrary_utils, builders::update_builder::ProposalBuilder},
    update::{SignedUpdateProposal, SignedUpdateVote, UpdateVote},
};
use chain_crypto::{Ed25519, Ed25519Extended, SecretKey};
use quickcheck::{Arbitrary, Gen};
use std::fmt::{self, Debug};
use std::{collections::HashMap, iter};

#[derive(Clone)]
pub struct UpdateProposalData {
    pub leaders: HashMap<LeaderId, SecretKey<Ed25519Extended>>,
    pub proposal: SignedUpdateProposal,
    pub proposal_id: Hash,
    pub votes: Vec<SignedUpdateVote>,
    pub block_signing_key: SecretKey<Ed25519>,
    pub update_successful: bool,
}

impl Debug for UpdateProposalData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let leaders: Vec<LeaderId> = self.leaders.keys().cloned().collect();
        f.debug_struct("UpdateProposalData")
            .field("leaders", &leaders)
            .field("proposal", &self.proposal)
            .field("proposal_id", &self.proposal_id)
            .field("votes", &self.votes)
            .finish()
    }
}

impl UpdateProposalData {
    pub fn leaders_ids(&self) -> Vec<LeaderId> {
        self.leaders.keys().cloned().collect()
    }

    pub fn proposal_settings(&self) -> ConfigParams {
        self.proposal.proposal.proposal.changes.clone()
    }
}

impl Arbitrary for UpdateProposalData {
    fn arbitrary<G: Gen>(gen: &mut G) -> Self {
        let leader_size = 1; //usize::arbitrary(gen) % 20 + 1;
        let leaders: HashMap<LeaderId, SecretKey<Ed25519Extended>> = iter::from_fn(|| {
            let sk: SecretKey<Ed25519Extended> = Arbitrary::arbitrary(gen);
            let leader_id = LeaderId(sk.to_public());
            Some((leader_id, sk))
        })
        .take(leader_size)
        .collect();

        let voters: HashMap<LeaderId, SecretKey<Ed25519Extended>> =
            arbitrary_utils::choose_random_map_subset(&leaders, gen);
        let leaders_ids: Vec<LeaderId> = leaders.keys().cloned().collect();
        let proposer_id = arbitrary_utils::choose_random_item(&leaders_ids, gen);

        //create proposal
        let unique_arbitrary_settings: Vec<ConfigParam> = vec![
            ConfigParam::SlotsPerEpoch(u32::arbitrary(gen)),
            ConfigParam::SlotDuration(u8::arbitrary(gen)),
            ConfigParam::EpochStabilityDepth(u32::arbitrary(gen)),
            ConfigParam::MaxNumberOfTransactionsPerBlock(u32::arbitrary(gen)),
            ConfigParam::BftSlotsRatio(Milli::arbitrary(gen)),
            ConfigParam::LinearFee(LinearFee::arbitrary(gen)),
            ConfigParam::ProposalExpiration(u32::arbitrary(gen)),
        ];

        let update_proposal = ProposalBuilder::new()
            .with_proposal_changes(arbitrary_utils::choose_random_vec_subset(
                &unique_arbitrary_settings,
                gen,
            ))
            .build();

        let signed_update_proposal = SignedProposalBuilder::new()
            .with_proposal_update(update_proposal)
            .with_proposer_id(proposer_id.clone())
            .build();

        //generate proposal header
        let proposal_id = Hash::arbitrary(gen);

        // create signed votes
        let signed_votes: Vec<SignedUpdateVote> = voters
            .iter()
            .map(|(id, _)| {
                let update_vote = UpdateVote {
                    proposal_id: proposal_id.clone(),
                    voter_id: id.clone(),
                };
                SignedUpdateVote { vote: update_vote }
            })
            .collect();

        let sk: chain_crypto::SecretKey<Ed25519> = Arbitrary::arbitrary(gen);
        let update_successful = signed_votes.len() > (leaders.len() / 2);

        UpdateProposalData {
            leaders: leaders,
            proposal: signed_update_proposal,
            proposal_id: proposal_id,
            votes: signed_votes,
            block_signing_key: sk,
            update_successful,
        }
    }
}
