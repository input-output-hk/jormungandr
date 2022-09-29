use crate::{builder::VotePlanKey, config::CommitteeTemplate};
use chain_impl_mockchain::certificate::Proposals;
use jormungandr_lib::interfaces::{BlockDate, TokenIdentifier};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct VotePlanTemplate {
    pub committees: Vec<CommitteeTemplate>,
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    #[serde(with = "jormungandr_lib::interfaces::serde_proposals")]
    pub proposals: Proposals,
    #[serde(
        with = "jormungandr_lib::interfaces::serde_committee_member_public_keys",
        default = "Vec::new"
    )]
    pub committee_member_public_keys: Vec<chain_vote::MemberPublicKey>,
    pub voting_token: TokenIdentifier,
    #[serde(flatten)]
    pub vote_plan_key: VotePlanKey,
    pub private: Option<PrivateParameters>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PrivateParameters {
    pub crs: Option<String>,
    pub threshold: Option<usize>,
}
