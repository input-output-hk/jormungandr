use crate::crypto::hash::Hash;
use crate::interfaces::blockdate::BlockDateDef;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub enum VoteOptions {
    OneOf { max_value: u8 }, // where max_value is up to 15
}

fn get_proposal_hash(proposal: &chain_impl_mockchain::certificate::Proposal) -> Hash {
    Hash::from(proposal.external_id().clone())
}

fn get_proposal_vote_options(
    proposal: &chain_impl_mockchain::certificate::Proposal,
) -> VoteOptions {
    VoteOptions::OneOf {
        max_value: proposal.options().as_byte(),
    }
}

#[derive(Serialize)]
#[serde(remote = "chain_impl_mockchain::certificate::Proposal")]
pub struct Proposal {
    #[serde(getter = "get_proposal_hash")]
    pub external_id: Hash,
    #[serde(getter = "get_proposal_vote_options")]
    pub options: VoteOptions,
}

#[derive(Serialize)]
pub struct ProposalSerializableHelper<'a>(
    #[serde(with = "Proposal")] pub &'a chain_impl_mockchain::certificate::Proposal,
);

fn serialize_proposals<S>(
    proposals: &chain_impl_mockchain::certificate::Proposals,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(proposals.len()))?;
    for e in proposals.iter() {
        seq.serialize_element(&ProposalSerializableHelper(e))?;
    }
    seq.end()
}

#[derive(Serialize)]
#[serde(remote = "chain_impl_mockchain::certificate::VotePlan")]
pub struct VotePlan {
    #[serde(
        with = "BlockDateDef",
        getter = "chain_impl_mockchain::certificate::VotePlan::vote_start"
    )]
    pub vote_start: chain_impl_mockchain::block::BlockDate,

    #[serde(
        with = "BlockDateDef",
        getter = "chain_impl_mockchain::certificate::VotePlan::vote_end"
    )]
    pub vote_end: chain_impl_mockchain::block::BlockDate,

    #[serde(
        with = "BlockDateDef",
        getter = "chain_impl_mockchain::certificate::VotePlan::committee_end"
    )]
    pub committee_end: chain_impl_mockchain::block::BlockDate,

    #[serde(
        serialize_with = "serialize_proposals",
        getter = "chain_impl_mockchain::certificate::VotePlan::proposals"
    )]
    pub proposals: chain_impl_mockchain::certificate::Proposals,
}

#[derive(Serialize)]
pub struct VotePlanSerializableHelper(
    #[serde(with = "VotePlan")] chain_impl_mockchain::certificate::VotePlan,
);

impl VotePlanSerializableHelper {
    pub fn new(vote_plan: chain_impl_mockchain::certificate::VotePlan) -> Self {
        Self(vote_plan)
    }
}

#[derive(Serialize)]
pub struct VotePlanWithId {
    pub voteplan_id: Hash,

    #[serde(with = "BlockDateDef")]
    pub vote_start: chain_impl_mockchain::block::BlockDate,

    #[serde(with = "BlockDateDef")]
    pub vote_end: chain_impl_mockchain::block::BlockDate,

    #[serde(with = "BlockDateDef")]
    pub committee_end: chain_impl_mockchain::block::BlockDate,

    #[serde(serialize_with = "serialize_proposals_index")]
    pub proposals: chain_impl_mockchain::certificate::Proposals,
}

impl VotePlanWithId {
    pub fn new(vote_plan: &chain_impl_mockchain::certificate::VotePlan) -> VotePlanWithId {
        VotePlanWithId {
            voteplan_id: Hash::from(vote_plan.to_id()),
            vote_start: vote_plan.vote_start(),
            vote_end: vote_plan.vote_end(),
            committee_end: vote_plan.committee_end(),
            proposals: vote_plan.proposals().clone(),
        }
    }
}

#[derive(Serialize)]
pub struct ProposalWithIndex {
    pub external_id: Hash,
    pub options: VoteOptions,
    pub index: u8,
}

fn serialize_proposals_index<S>(
    proposals: &chain_impl_mockchain::certificate::Proposals,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(proposals.len()))?;
    for (i, e) in proposals.iter().enumerate() {
        seq.serialize_element(&ProposalWithIndex {
            external_id: get_proposal_hash(e),
            options: get_proposal_vote_options(e),
            index: i as u8,
        })?;
    }
    seq.end()
}
