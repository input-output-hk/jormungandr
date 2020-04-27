use serde::Serialize;
#[derive(Serialize)]
#[serde(remote = "chain_impl_mockchain::block::BlockDate")]
struct BlockDate {
    pub epoch: u32,
    pub slot_id: u32,
}

#[derive(Serialize)]
#[serde(remote = "chain_impl_mockchain::certificate::ExternalProposalId")]
struct ExternalProposalId {
    #[serde(getter = "chain_impl_mockchain::certificate::ExternalProposalId::to_string")]
    id: String,
}

#[derive(Serialize)]
#[serde(remote = "chain_impl_mockchain::certificate::VoteOptions")]
struct VoteOptions {
    #[serde(getter = "chain_impl_mockchain::certificate::VoteOptions::as_byte")]
    num_choices: u8,
}

#[derive(Serialize)]
// #[serde(remote="chain_impl_mockchain::certificate::Proposal")]
pub struct Proposal {
    // #[serde(with="ExternalProposalId", getter="chain_impl_mockchain::certificate::Proposal::external_id")]
    pub external_id: String,
    // #[serde(with="VoteOptions", getter="chain_impl_mockchain::certificate::Proposal::options")]
    pub options: u8,
}

impl Proposal {
    fn new(p: &chain_impl_mockchain::certificate::Proposal) -> Self {
        Self {
            external_id: p.external_id().to_string(),
            options: p.options().as_byte(),
        }
    }
}

#[derive(Serialize)]
pub struct VotePlan {
    /// the vote start validity
    #[serde(with = "BlockDate")]
    pub vote_start: chain_impl_mockchain::block::BlockDate,
    /// the duration within which it is possible to vote for one of the proposals
    /// of this voting plan.
    #[serde(with = "BlockDate")]
    pub vote_end: chain_impl_mockchain::block::BlockDate,
    /// the committee duration is the time allocated to the committee to open
    /// the ballots and publish the results on chain
    #[serde(with = "BlockDate")]
    pub committee_end: chain_impl_mockchain::block::BlockDate,
    /// the proposals to vote for
    pub proposals: Vec<Proposal>,
}

impl VotePlan {
    pub fn from_vote_plan(plan: &chain_impl_mockchain::certificate::VotePlan) -> Self {
        VotePlan {
            vote_start: plan.vote_start(),
            vote_end: plan.vote_end(),
            committee_end: plan.committee_end(),
            proposals: plan.proposals().iter().map(Proposal::new).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct VotePlans {
    plans: Vec<VotePlan>,
}

impl VotePlans {
    pub fn new(plans: Vec<VotePlan>) -> Self {
        Self { plans }
    }
}
