use crate::jcli_lib::utils::io;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{serde_base64_bytes, VotePlanStatus},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VotePlanError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("could not decode vote plan")]
    VotePlansRead,
    #[error("could not find vote plan with specified id")]
    VotePlanIdNotFound,
    #[error("please specify a correct id for the vote plan")]
    UnclearVotePlan,
}

// Read json-encoded vote plan(s) from file and returns the one
// with the specified id. If there is only one vote plan in the input
// the id can be
pub fn get_vote_plan_by_id<P: AsRef<Path>>(
    vote_plan_file: Option<P>,
    id: Option<&Hash>,
) -> Result<VotePlanStatus, VotePlanError> {
    let value: Value = serde_json::from_reader(io::open_file_read(&vote_plan_file)?)?;
    match value {
        Value::Array(vote_plans) => {
            let plans = vote_plans
                .into_iter()
                .map(serde_json::from_value)
                .collect::<Result<Vec<VotePlanStatus>, serde_json::Error>>()?;
            match id {
                Some(id) => plans
                    .into_iter()
                    .find(|plan| &plan.id == id)
                    .ok_or(VotePlanError::VotePlanIdNotFound),
                None if plans.len() == 1 => Ok(plans.into_iter().next().unwrap()),
                _ => Err(VotePlanError::UnclearVotePlan),
            }
        }
        obj @ Value::Object(_) => {
            let vote_plan: VotePlanStatus = serde_json::from_value(obj)?;
            match id {
                None => Ok(vote_plan),
                Some(id) if &vote_plan.id == id => Ok(vote_plan),
                _ => Err(VotePlanError::VotePlanIdNotFound),
            }
        }
        _ => Err(VotePlanError::VotePlansRead),
    }
}

#[derive(Debug, Error)]
pub enum SharesError {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("shares cannot be empty")]
    Empty,
    #[error("proposals have different number of shares")]
    ProposalSharesNotBalanced,
    #[error("insufficient amount of shares for vote plan decryption")]
    InsufficientShares,
    #[error("invalid binary share data")]
    InvalidBinaryShare,
    #[error("decryption share is not valid")]
    ValidationFailed(#[from] chain_vote::tally::DecryptionError),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TallyDecryptShare(#[serde(with = "serde_base64_bytes")] Vec<u8>);

// Set of shares (belonging to a single committee member) for the decryption of a vote plan
#[derive(Debug, Serialize, Deserialize)]
pub struct MemberVotePlanShares(Vec<TallyDecryptShare>);

// Set of decrypt shares (belonging to different committee members)
// that decrypts a vote plan
#[derive(Debug, Serialize, Deserialize)]
pub struct VotePlanDecryptShares(Vec<Vec<TallyDecryptShare>>);

impl TryFrom<TallyDecryptShare> for chain_vote::TallyDecryptShare {
    type Error = SharesError;

    fn try_from(value: TallyDecryptShare) -> Result<Self, Self::Error> {
        chain_vote::TallyDecryptShare::from_bytes(&value.0).ok_or(SharesError::InvalidBinaryShare)
    }
}

impl From<Vec<chain_vote::TallyDecryptShare>> for MemberVotePlanShares {
    fn from(shares: Vec<chain_vote::TallyDecryptShare>) -> Self {
        Self(
            shares
                .into_iter()
                .map(|s| TallyDecryptShare(s.to_bytes()))
                .collect::<Vec<_>>(),
        )
    }
}

impl TryFrom<Vec<MemberVotePlanShares>> for VotePlanDecryptShares {
    type Error = SharesError;
    fn try_from(shares: Vec<MemberVotePlanShares>) -> Result<Self, Self::Error> {
        let shares = shares.into_iter().map(|s| s.0).collect::<Vec<_>>();
        if shares.is_empty() {
            return Err(SharesError::Empty);
        }
        let mut res = vec![Vec::new(); shares[0].len()];
        // transponse 2d array
        for member_shares in shares {
            if member_shares.len() != res.len() {
                return Err(SharesError::ProposalSharesNotBalanced);
            }
            for (i, share) in member_shares.into_iter().enumerate() {
                res[i].push(share);
            }
        }
        Ok(VotePlanDecryptShares(res))
    }
}

impl TryFrom<VotePlanDecryptShares> for Vec<Vec<chain_vote::TallyDecryptShare>> {
    type Error = SharesError;
    fn try_from(vote_plan: VotePlanDecryptShares) -> Result<Self, Self::Error> {
        vote_plan
            .0
            .into_iter()
            .map(|v| {
                v.into_iter()
                    .map(chain_vote::TallyDecryptShare::try_from)
                    .collect::<Result<Vec<_>, Self::Error>>()
            })
            .collect::<Result<Vec<_>, Self::Error>>()
    }
}

pub fn read_vote_plan_shares_from_file<P: AsRef<Path>>(
    share_path: Option<P>,
    proposals: usize,
    threshold: Option<usize>,
) -> Result<VotePlanDecryptShares, SharesError> {
    let vote_plan_shares: VotePlanDecryptShares =
        serde_json::from_reader(io::open_file_read(&share_path)?)?;
    if vote_plan_shares.0.len() != proposals || vote_plan_shares.0[0].len() < threshold.unwrap_or(1)
    {
        return Err(SharesError::InsufficientShares);
    }

    Ok(vote_plan_shares)
}
