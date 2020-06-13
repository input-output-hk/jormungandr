use crate::{
    crypto::hash::Hash,
    interfaces::{account_identifier::AccountIdentifier, blockdate::BlockDateDef},
};
use chain_impl_mockchain::{
    header::BlockDate,
    vote::{self, PayloadType},
};
use core::ops::Range;
use serde::Serialize;
use std::collections::HashMap;

#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, serde::Deserialize,
)]
#[serde(remote = "PayloadType")]
pub enum PayloadTypeDef {
    Public,
}

#[derive(Serialize)]
pub struct VotePlanStatus {
    pub id: Hash,
    #[serde(with = "PayloadTypeDef")]
    pub payload: PayloadType,
    #[serde(with = "BlockDateDef")]
    pub vote_start: BlockDate,
    #[serde(with = "BlockDateDef")]
    pub vote_end: BlockDate,
    #[serde(with = "BlockDateDef")]
    pub committee_end: BlockDate,
    pub proposals: Vec<VoteProposalStatus>,
}

#[derive(Serialize)]
pub enum Tally {
    Public { result: TallyResult },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TallyResult {
    results: Vec<u64>,

    options: Range<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum Payload {
    Public { choice: u8 },
}

#[derive(Serialize)]
pub struct VoteProposalStatus {
    pub index: u8,
    pub proposal_id: Hash,
    pub options: Range<u8>,
    pub tally: Option<Tally>,
    pub votes: HashMap<AccountIdentifier, Payload>,
}

impl From<vote::Payload> for Payload {
    fn from(this: vote::Payload) -> Self {
        match this {
            vote::Payload::Public { choice } => Self::Public {
                choice: choice.as_byte(),
            },
        }
    }
}

impl From<vote::TallyResult> for TallyResult {
    fn from(this: vote::TallyResult) -> Self {
        Self {
            results: this.results().iter().map(|v| (*v).into()).collect(),
            options: this.options().choice_range().clone(),
        }
    }
}

impl From<vote::Tally> for Tally {
    fn from(this: vote::Tally) -> Self {
        match this {
            vote::Tally::Public { result } => Tally::Public {
                result: result.into(),
            },
        }
    }
}

impl From<vote::VoteProposalStatus> for VoteProposalStatus {
    fn from(this: vote::VoteProposalStatus) -> Self {
        Self {
            index: this.index,
            proposal_id: this.proposal_id.into(),
            options: this.options.choice_range().clone(),
            tally: this.tally.map(|t| t.into()),
            votes: this
                .votes
                .iter()
                .map(|(k, p)| (k.clone().into(), p.clone().into()))
                .collect(),
        }
    }
}

impl From<vote::VotePlanStatus> for VotePlanStatus {
    fn from(this: vote::VotePlanStatus) -> Self {
        Self {
            id: this.id.into(),
            vote_start: this.vote_start,
            vote_end: this.vote_end,
            committee_end: this.committee_end,
            payload: this.payload,
            proposals: this.proposals.into_iter().map(|p| p.into()).collect(),
        }
    }
}
