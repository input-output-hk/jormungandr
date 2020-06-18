use crate::{
    crypto::hash::Hash,
    interfaces::{account_identifier::AccountIdentifier, blockdate::BlockDateDef, value::ValueDef},
};
use chain_impl_mockchain::{
    certificate::{ExternalProposalId, Proposal, Proposals, VoteAction, VotePlan},
    header::BlockDate,
    ledger::governance::TreasuryGovernanceAction,
    value::Value,
    vote::{self, Options, PayloadType},
};
use core::ops::Range;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(
    Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, serde::Deserialize,
)]
#[serde(remote = "PayloadType", rename_all = "snake_case")]
enum PayloadTypeDef {
    Public,
}

#[derive(Deserialize)]
#[serde(remote = "VotePlan")]
pub struct VotePlanDef {
    #[serde(with = "PayloadTypeDef", getter = "payload_type")]
    payload_type: PayloadType,
    #[serde(with = "BlockDateDef", getter = "vote_start")]
    vote_start: BlockDate,
    #[serde(with = "BlockDateDef", getter = "vote_end")]
    vote_end: BlockDate,
    #[serde(with = "BlockDateDef", getter = "committee_end")]
    committee_end: BlockDate,
    #[serde(deserialize_with = "deserialize_proposals", getter = "proposals")]
    proposals: Proposals,
}

#[derive(Deserialize)]
#[serde(remote = "Proposal")]
struct VoteProposalDef {
    #[serde(
        deserialize_with = "deserialize_external_proposal_id",
        getter = "external_id"
    )]
    external_id: ExternalProposalId,
    #[serde(deserialize_with = "deserialize_choices", getter = "options")]
    options: Options,
    #[serde(with = "VoteActionDef", getter = "action")]
    action: VoteAction,
}

#[derive(Deserialize)]
#[serde(remote = "VoteAction", rename_all = "snake_case")]
enum VoteActionDef {
    OffChain,
    #[serde(with = "TreasuryGovernanceActionDef")]
    Treasury {
        action: TreasuryGovernanceAction,
    },
}

#[derive(Deserialize)]
#[serde(remote = "TreasuryGovernanceAction", rename_all = "snake_case")]
enum TreasuryGovernanceActionDef {
    TransferToRewards {
        #[serde(with = "ValueDef")]
        value: Value,
    },
}

impl From<VotePlanDef> for VotePlan {
    fn from(vpd: VotePlanDef) -> Self {
        Self::new(
            vpd.vote_start,
            vpd.vote_end,
            vpd.committee_end,
            vpd.proposals,
            vpd.payload_type,
        )
    }
}

impl From<VoteProposalDef> for Proposal {
    fn from(vpd: VoteProposalDef) -> Self {
        Self::new(vpd.external_id, vpd.options, vpd.action)
    }
}

fn deserialize_external_proposal_id<'de, D>(deserializer: D) -> Result<ExternalProposalId, D::Error>
where
    D: Deserializer<'de>,
{
    struct ExternalProposalIdVisitor;

    impl<'de> serde::de::Visitor<'de> for ExternalProposalIdVisitor {
        type Value = ExternalProposalId;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("external proposal id in a hexadecimal form")
        }

        fn visit_str<E>(self, value: &str) -> Result<ExternalProposalId, E>
        where
            E: serde::de::Error,
        {
            Ok(std::str::FromStr::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_str(ExternalProposalIdVisitor)
}

fn deserialize_choices<'de, D>(deserializer: D) -> Result<Options, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionsVisitor;

    impl<'de> serde::de::Visitor<'de> for OptionsVisitor {
        type Value = Options;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("options number must be an integer less than 256")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Options, E>
        where
            E: serde::de::Error,
        {
            if value > 255 {
                return Err(serde::de::Error::custom("expecting a value less than 256"));
            }
            Options::new_length(value as u8).map_err(|err| serde::de::Error::custom(err))
        }
    }

    deserializer.deserialize_u64(OptionsVisitor)
}

fn deserialize_proposals<'de, D>(deserializer: D) -> Result<Proposals, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct ProposalInternal(#[serde(with = "VoteProposalDef")] Proposal);

    #[derive(Deserialize)]
    struct ProposalsList(Vec<ProposalInternal>);

    let proposals_list = ProposalsList::deserialize(deserializer)?;
    let mut proposals = Proposals::new();
    for proposal in proposals_list.0.into_iter() {
        match proposals.push(proposal.0) {
            chain_impl_mockchain::certificate::PushProposal::Full { .. } => {
                panic!("too much proposals")
            }
            _ => {}
        }
    }
    Ok(proposals)
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
