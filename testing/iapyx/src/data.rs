use chain_impl_mockchain::{
    certificate::VotePlanId,
    vote::{Options, PayloadType},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, fmt, str};
pub use wallet_core::{Choice, Value};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Fund {
    //  pub id: i32,
    #[serde(alias = "fundName")]
    pub fund_name: String,
    #[serde(alias = "fundGoal")]
    pub fund_goal: String,
    #[serde(alias = "votingPowerInfo")]
    pub voting_power_info: String,
    #[serde(alias = "rewardsInfo")]
    pub rewards_info: String,
    #[serde(alias = "fundStartTime")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub fund_start_time: i64,
    #[serde(alias = "fundEndTime")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub fund_end_time: i64,
    #[serde(alias = "nextFundStartTime")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub next_fund_start_time: i64,
    #[serde(alias = "chainVotePlans")]
    pub chain_vote_plans: Vec<Voteplan>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Voteplan {
    pub id: i32,
    #[serde(alias = "chainVoteplanId")]
    pub chain_voteplan_id: String,
    #[serde(alias = "chainVoteStartTime")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub chain_vote_start_time: i64,
    #[serde(alias = "chainVoteEndTime")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub chain_vote_end_time: i64,
    #[serde(alias = "chainCommitteeEnd")]
    #[serde(serialize_with = "crate::utils::serde::serialize_unix_timestamp_as_rfc3339")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_unix_timestamp_from_rfc3339")]
    pub chain_committee_end: i64,
    #[serde(alias = "chainVoteplanPayload")]
    pub chain_voteplan_payload: String,
    #[serde(alias = "fundId")]
    pub fund_id: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Category {
    #[serde(alias = "categoryId")]
    pub category_id: String,
    #[serde(alias = "categoryName")]
    pub category_name: String,
    #[serde(alias = "categoryDescription")]
    pub category_description: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Proposer {
    #[serde(alias = "proposerName")]
    pub proposer_name: String,
    #[serde(alias = "proposerEmail")]
    pub proposer_email: String,
    #[serde(alias = "proposerUrl")]
    pub proposer_url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Proposal {
    #[serde(alias = "internalId")]
    pub internal_id: String,
    #[serde(alias = "proposalId")]
    pub proposal_id: String,
    #[serde(alias = "category")]
    pub proposal_category: Category,
    #[serde(alias = "proposalTitle")]
    pub proposal_title: String,
    #[serde(alias = "proposalSummary")]
    pub proposal_summary: String,
    #[serde(alias = "proposalProblem")]
    pub proposal_problem: String,
    #[serde(alias = "proposalSolution")]
    pub proposal_solution: String,
    #[serde(alias = "proposalPublicKey")]
    pub proposal_public_key: String,
    #[serde(alias = "proposalFunds")]
    pub proposal_funds: i64,
    #[serde(alias = "proposalUrl")]
    pub proposal_url: String,
    #[serde(alias = "proposalFilesUrl")]
    pub proposal_files_url: String,
    pub proposer: Proposer,
    #[serde(alias = "chainProposalId")]
    #[serde(serialize_with = "crate::utils::serde::serialize_bin_as_str")]
    #[serde(deserialize_with = "crate::utils::serde::deserialize_string_as_bytes")]
    pub chain_proposal_id: Vec<u8>,
    #[serde(alias = "chainProposalIndex")]
    pub chain_proposal_index: i64,
    #[serde(alias = "chainVoteOptions")]
    pub chain_vote_options: VoteOptions,
    #[serde(alias = "chainVoteplanId")]
    pub chain_voteplan_id: String,
    #[serde(alias = "chainVoteplanPayload")]
    pub chain_voteplan_payload: String,
}

impl Proposal {
    pub fn chain_proposal_id_as_str(&self) -> String {
        str::from_utf8(&self.chain_proposal_id).unwrap().to_string()
    }

    pub fn get_option_text(&self, choice: u8) -> String {
        self.chain_vote_options
            .0
            .iter()
            .find(|(_, y)| **y == choice)
            .map(|(x, _)| x.to_string())
            .unwrap()
    }
}

impl Into<wallet_core::Proposal> for Proposal {
    fn into(self) -> wallet_core::Proposal {
        let chain_proposal_index = self.chain_proposal_index as u8;
        let bytes = hex::decode(self.chain_voteplan_id).unwrap();
        let mut vote_plan_id = [0; 32];
        let bytes = &bytes[..vote_plan_id.len()]; // panics if not enough data
        vote_plan_id.copy_from_slice(bytes);

        wallet_core::Proposal::new(
            VotePlanId::try_from(vote_plan_id).unwrap(),
            PayloadType::Public,
            chain_proposal_index,
            Options::new_length(self.chain_vote_options.0.len() as u8).unwrap(),
        )
    }
}

pub struct SimpleVoteStatus {
    pub chain_proposal_id: String,
    pub proposal_title: String,
    pub choice: String,
}

impl fmt::Display for SimpleVoteStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "# {}, '{}' -> Choice:  {}",
            self.chain_proposal_id, self.proposal_title, self.choice
        )
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct VoteOptions(pub VoteOptionsMap);
pub type VoteOptionsMap = HashMap<String, u8>;
