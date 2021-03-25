use crate::{
    crypto::hash::Hash,
    interfaces::{blockdate::BlockDateDef, stake::Stake, value::ValueDef},
};
use bech32::{FromBase32, ToBase32};
use chain_impl_mockchain::{
    certificate::{ExternalProposalId, Proposal, Proposals, VoteAction, VotePlan},
    header::BlockDate,
    ledger::governance::{ParametersGovernanceAction, TreasuryGovernanceAction},
    value::Value,
    vote::{self, Options, PayloadType},
};
use chain_vote::MemberPublicKey;
use core::ops::Range;
use serde::de::Visitor;
use serde::ser::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::TryInto;
use std::str;
use vote::{Choice, Weight};

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(remote = "PayloadType", rename_all = "snake_case")]
enum PayloadTypeDef {
    Public,
    Private,
}

struct SerdeMemberPublicKey(chain_vote::MemberPublicKey);

pub const MEMBER_PUBLIC_KEY_BECH32_HRP: &str = "p256k1_memberpk";

impl<'de> Deserialize<'de> for SerdeMemberPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        struct Bech32Visitor;
        impl<'de> Visitor<'de> for Bech32Visitor {
            type Value = SerdeMemberPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(
                    formatter,
                    "a Bech32 representation of member public key with prefix {}",
                    MEMBER_PUBLIC_KEY_BECH32_HRP
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(value.to_string())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let (hrp, content) = bech32::decode(&v).map_err(|err| {
                    serde::de::Error::custom(format!(
                        "Invalid public key bech32 representation {}, with err: {}",
                        &v, err
                    ))
                })?;

                let content = Vec::<u8>::from_base32(&content).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Invalid public key bech32 representation {}, with err: {}",
                        &v, e
                    ))
                })?;

                if hrp != MEMBER_PUBLIC_KEY_BECH32_HRP {
                    return Err(serde::de::Error::custom(format!(
                        "Invalid public key bech32 public hrp {}, expecting {}",
                        hrp, MEMBER_PUBLIC_KEY_BECH32_HRP,
                    )));
                }

                Ok(SerdeMemberPublicKey(
                    MemberPublicKey::from_bytes(&content).ok_or_else(|| {
                        serde::de::Error::custom(format!(
                            "Invalid public key with bech32 representation {}",
                            &v
                        ))
                    })?,
                ))
            }
        }

        struct BytesVisitor;
        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = SerdeMemberPublicKey;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("binary data for member public key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let pk = MemberPublicKey::from_bytes(v).ok_or_else(|| {
                    serde::de::Error::custom("Invalid binary data for member public key")
                })?;
                Ok(SerdeMemberPublicKey(pk))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_string(Bech32Visitor)
        } else {
            deserializer.deserialize_bytes(BytesVisitor)
        }
    }
}

impl Serialize for SerdeMemberPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(
                &bech32::encode(MEMBER_PUBLIC_KEY_BECH32_HRP, &self.0.to_bytes().to_base32())
                    .map_err(|e| <S as Serializer>::Error::custom(format!("{}", e)))?,
            )
        } else {
            serializer.serialize_bytes(&self.0.to_bytes())
        }
    }
}

fn committee_keys(v: &VotePlan) -> Vec<chain_vote::MemberPublicKey> {
    v.committee_public_keys().to_vec()
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "VotePlan")]
pub struct VotePlanDef {
    #[serde(with = "PayloadTypeDef", getter = "VotePlan::payload_type")]
    payload_type: PayloadType,
    #[serde(with = "BlockDateDef", getter = "VotePlan::vote_start")]
    vote_start: BlockDate,
    #[serde(with = "BlockDateDef", getter = "VotePlan::vote_end")]
    vote_end: BlockDate,
    #[serde(with = "BlockDateDef", getter = "VotePlan::committee_end")]
    committee_end: BlockDate,
    #[serde(with = "serde_proposals", getter = "VotePlan::proposals")]
    proposals: Proposals,
    #[serde(
        with = "serde_committee_member_public_keys",
        getter = "committee_keys",
        default = "Vec::new"
    )]
    committee_member_public_keys: Vec<chain_vote::MemberPublicKey>,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "Proposal")]
struct VoteProposalDef {
    #[serde(with = "serde_external_proposal_id", getter = "Proposal::external_id")]
    external_id: ExternalProposalId,
    #[serde(with = "serde_choices", getter = "Proposal::options")]
    options: Options,
    #[serde(with = "VoteActionDef", getter = "Proposal::action")]
    action: VoteAction,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "VoteAction", rename_all = "snake_case")]
enum VoteActionDef {
    OffChain,
    #[serde(with = "TreasuryGovernanceActionDef")]
    Treasury {
        action: TreasuryGovernanceAction,
    },
    #[serde(with = "ParametersGovernanceActionDef")]
    Parameters {
        action: ParametersGovernanceAction,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "ParametersGovernanceAction", rename_all = "snake_case")]
enum ParametersGovernanceActionDef {
    RewardAdd {
        #[serde(with = "ValueDef")]
        value: Value,
    },
    NoOp,
}

#[derive(Deserialize, Serialize)]
#[serde(remote = "TreasuryGovernanceAction", rename_all = "snake_case")]
enum TreasuryGovernanceActionDef {
    TransferToRewards {
        #[serde(with = "ValueDef")]
        value: Value,
    },
    NoOp,
}

impl From<VotePlanDef> for VotePlan {
    fn from(vpd: VotePlanDef) -> Self {
        Self::new(
            vpd.vote_start,
            vpd.vote_end,
            vpd.committee_end,
            vpd.proposals,
            vpd.payload_type,
            vpd.committee_member_public_keys,
        )
    }
}

mod serde_committee_member_public_keys {
    use crate::interfaces::vote::SerdeMemberPublicKey;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Vec<chain_vote::MemberPublicKey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PublicKeysSeqVisitor;
        impl<'de> Visitor<'de> for PublicKeysSeqVisitor {
            type Value = Vec<SerdeMemberPublicKey>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of member public keys")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, <A as SeqAccess<'de>>::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(key) = seq.next_element()? {
                    result.push(key);
                }
                Ok(result)
            }
        }
        let keys = deserializer.deserialize_seq(PublicKeysSeqVisitor {})?;
        Ok(keys.iter().map(|key| key.0.clone()).collect())
    }

    pub fn serialize<S>(
        keys: &[chain_vote::MemberPublicKey],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(keys.len()))?;
        for key in keys {
            seq.serialize_element(&SerdeMemberPublicKey(key.clone()))?;
        }
        seq.end()
    }
}

impl From<VoteProposalDef> for Proposal {
    fn from(vpd: VoteProposalDef) -> Self {
        Self::new(vpd.external_id, vpd.options, vpd.action)
    }
}

mod serde_external_proposal_id {
    use super::*;
    use serde::{Deserializer, Serialize, Serializer};
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ExternalProposalId, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = ExternalProposalId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an external proposal id in hexadecimal form")
            }

            fn visit_str<E>(self, value: &str) -> Result<ExternalProposalId, E>
            where
                E: Error,
            {
                str::parse(value).map_err(Error::custom)
            }
        }

        struct BinaryVisitor;

        impl<'de> Visitor<'de> for BinaryVisitor {
            type Value = ExternalProposalId;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an external proposal id in the binary form")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<ExternalProposalId, E>
            where
                E: Error,
            {
                value.try_into().map_err(Error::custom)
            }
        }
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(StringVisitor)
        } else {
            deserializer.deserialize_bytes(BinaryVisitor)
        }
    }

    pub fn serialize<S>(id: &ExternalProposalId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            id.to_string().serialize(serializer)
        } else {
            id.as_ref().serialize(serializer)
        }
    }
}

mod serde_choices {
    use super::*;
    use serde::{Deserializer, Serialize, Serializer};
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Options, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OptionsVisitor;

        impl<'de> serde::de::Visitor<'de> for OptionsVisitor {
            type Value = Options;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number of options from 0 to 255")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Options, E>
            where
                E: serde::de::Error,
            {
                if value > 255 {
                    return Err(serde::de::Error::custom("expecting a value less than 256"));
                }
                Options::new_length(value as u8).map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_u64(OptionsVisitor)
    }

    pub fn serialize<S>(options: &Options, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let v = options.choice_range().end as u64;
        v.serialize(serializer)
    }
}

mod serde_proposals {
    use super::*;
    use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
    #[derive(Deserialize, Serialize)]
    struct ProposalInternal(#[serde(with = "VoteProposalDef")] Proposal);

    #[derive(Deserialize)]
    struct ProposalsList(Vec<ProposalInternal>);

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Proposals, D::Error>
    where
        D: Deserializer<'de>,
    {
        let proposals_list = ProposalsList::deserialize(deserializer)?;
        let mut proposals = Proposals::new();
        for proposal in proposals_list.0.into_iter() {
            if let chain_impl_mockchain::certificate::PushProposal::Full { .. } =
                proposals.push(proposal.0)
            {
                panic!("too many proposals")
            }
        }
        Ok(proposals)
    }

    pub fn serialize<S>(proposals: &Proposals, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use std::ops::Deref;
        let v = proposals.deref();
        let mut seq = serializer.serialize_seq(Some(v.len()))?;
        for prop in v {
            let prop = ProposalInternal(prop.clone());
            seq.serialize_element(&prop)?;
        }
        seq.end()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    #[serde(with = "serde_committee_member_public_keys")]
    pub committee_member_keys: Vec<MemberPublicKey>,
    pub proposals: Vec<VoteProposalStatus>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Tally {
    Public { result: TallyResult },
    Private { state: PrivateTallyState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TallyResult {
    results: Vec<u64>,
    options: Range<u8>,
}

impl TallyResult {
    pub fn results(&self) -> Vec<u64> {
        self.results.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedTally(#[serde(with = "serde_base64_bytes")] Vec<u8>);

impl EncryptedTally {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

pub mod serde_base64_bytes {
    use serde::de::{Error, Visitor};
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ByteStringVisitor;
        impl<'de> Visitor<'de> for ByteStringVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("base64 encoded binary data")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                base64::decode(v).map_err(|e| E::custom(format!("{}", e)))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                self.visit_str(&v)
            }
        }

        struct ByteArrayVisitor;
        impl<'de> Visitor<'de> for ByteArrayVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("binary data")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v.to_vec())
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_string(ByteStringVisitor {})
        } else {
            deserializer.deserialize_bytes(ByteArrayVisitor {})
        }
    }

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&base64::encode(bytes))
        } else {
            serializer.serialize_bytes(bytes)
        }
    }
}

impl From<EncryptedTally> for chain_vote::EncryptedTally {
    fn from(encrypted_tally: EncryptedTally) -> chain_vote::EncryptedTally {
        chain_vote::EncryptedTally::from_bytes(&encrypted_tally.0).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrivateTallyState {
    Encrypted {
        encrypted_tally: EncryptedTally,
        total_stake: Stake,
    },
    Decrypted {
        result: TallyResult,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Payload {
    Public {
        choice: u8,
    },
    Private {
        #[serde(with = "serde_base64_bytes")]
        encrypted_vote: Vec<u8>,
        #[serde(with = "serde_base64_bytes")]
        proof: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VoteProposalStatus {
    pub index: u8,
    pub proposal_id: Hash,
    pub options: Range<u8>,
    pub tally: Option<Tally>,
    pub votes_cast: usize,
}

impl From<vote::Payload> for Payload {
    fn from(this: vote::Payload) -> Self {
        match this {
            vote::Payload::Public { choice } => Self::Public {
                choice: choice.as_byte(),
            },
            vote::Payload::Private {
                encrypted_vote,
                proof,
            } => Self::Private {
                encrypted_vote: encrypted_vote.serialize().into(),
                proof: proof.serialize().into(),
            },
        }
    }
}

impl Payload {
    pub fn choice(&self) -> Option<u8> {
        match self {
            Payload::Public { choice } => Some(*choice),
            Payload::Private { .. } => None,
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

impl From<chain_vote::Tally> for TallyResult {
    fn from(this: chain_vote::Tally) -> Self {
        Self {
            results: this.votes.iter().copied().collect(),
            options: 0..this.votes.len() as u8,
        }
    }
}

impl From<TallyResult> for vote::TallyResult {
    fn from(tally_result: TallyResult) -> vote::TallyResult {
        let mut result = vote::TallyResult::new(
            Options::new_length(tally_result.options.end - tally_result.options.start).unwrap(),
        );

        for (idx, value) in tally_result.results().iter().enumerate() {
            let weight: Weight = (*value).into();
            result.add_vote(Choice::new(idx as u8), weight).unwrap()
        }
        result
    }
}

impl From<vote::Tally> for Tally {
    fn from(this: vote::Tally) -> Self {
        match this {
            vote::Tally::Public { result } => Tally::Public {
                result: result.into(),
            },
            vote::Tally::Private { state } => Tally::Private {
                state: match state {
                    vote::PrivateTallyState::Encrypted {
                        encrypted_tally,
                        total_stake,
                    } => PrivateTallyState::Encrypted {
                        encrypted_tally: EncryptedTally(encrypted_tally.to_bytes()),
                        total_stake: total_stake.into(),
                    },
                    vote::PrivateTallyState::Decrypted { result } => PrivateTallyState::Decrypted {
                        result: result.into(),
                    },
                },
            },
        }
    }
}

impl From<Tally> for vote::Tally {
    fn from(tally: Tally) -> vote::Tally {
        match tally {
            Tally::Public { result } => vote::Tally::Public {
                result: result.into(),
            },
            Tally::Private { state } => vote::Tally::Private {
                state: match state {
                    PrivateTallyState::Encrypted {
                        encrypted_tally,
                        total_stake,
                    } => vote::PrivateTallyState::Encrypted {
                        encrypted_tally: encrypted_tally.into(),
                        total_stake: total_stake.into(),
                    },
                    PrivateTallyState::Decrypted { result } => vote::PrivateTallyState::Decrypted {
                        result: result.into(),
                    },
                },
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
            votes_cast: this.votes.size(),
        }
    }
}

impl From<VoteProposalStatus> for vote::VoteProposalStatus {
    fn from(vote_proposal_status: VoteProposalStatus) -> vote::VoteProposalStatus {
        vote::VoteProposalStatus {
            index: vote_proposal_status.index,
            proposal_id: vote_proposal_status.proposal_id.into(),
            options: Options::new_length(
                vote_proposal_status.options.end - vote_proposal_status.options.start,
            )
            .unwrap(),
            tally: vote_proposal_status.tally.map(|t| t.into()),
            votes: Default::default(),
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
            committee_member_keys: this.committee_public_keys,
            proposals: this.proposals.into_iter().map(|p| p.into()).collect(),
        }
    }
}

impl From<VotePlanStatus> for vote::VotePlanStatus {
    fn from(vote_plan_status: VotePlanStatus) -> vote::VotePlanStatus {
        vote::VotePlanStatus {
            id: vote_plan_status.id.into(),
            vote_start: vote_plan_status.vote_start,
            vote_end: vote_plan_status.vote_end,
            committee_end: vote_plan_status.committee_end,
            payload: vote_plan_status.payload,
            committee_public_keys: vote_plan_status.committee_member_keys,
            proposals: vote_plan_status
                .proposals
                .into_iter()
                .map(|p| p.into())
                .collect(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interfaces::vote::{serde_committee_member_public_keys, SerdeMemberPublicKey};
    use bech32::ToBase32;
    use chain_impl_mockchain::block::BlockDate;
    use chain_impl_mockchain::certificate::VotePlan;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn test_deserialize_member_public_keys() {
        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let crs = chain_vote::CRS::random(&mut rng);
        let comm_key = chain_vote::MemberCommunicationKey::new(&mut rng);

        let member_key =
            chain_vote::MemberState::new(&mut rng, 1, &crs, &[comm_key.to_public()], 0);
        let pk = member_key.public_key();
        let pks = vec![bech32::encode("p256k1_memberpk", pk.to_bytes().to_base32()).unwrap()];
        let json = serde_json::to_string(&pks).unwrap();

        let result: Vec<SerdeMemberPublicKey> = serde_json::from_str(&json).unwrap();
        assert_eq!(result[0].0, pk);

        let mut json_deserializer = serde_json::Deserializer::from_str(&json);
        let result =
            serde_committee_member_public_keys::deserialize(&mut json_deserializer).unwrap();
        assert_eq!(result[0], pk);
    }

    #[test]
    fn test_deserialize_vote_plan_def() {
        #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
        struct Helper(#[serde(with = "VotePlanDef")] VotePlan);

        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let crs = chain_vote::CRS::random(&mut rng);
        let comm_key = chain_vote::MemberCommunicationKey::new(&mut rng);

        let member_key =
            chain_vote::MemberState::new(&mut rng, 1, &crs, &[comm_key.to_public()], 0)
                .public_key();
        let bd = "42.12".parse::<BlockDate>().unwrap();
        let id = ExternalProposalId::from([0; 32]);
        let prop = Proposal::new(id, Options::new_length(1).unwrap(), VoteAction::OffChain);
        let mut proposals = Proposals::new();
        let _ = proposals.push(prop);
        let vote_plan = Helper(VotePlan::new(
            bd,
            bd,
            bd,
            proposals,
            PayloadType::Private,
            vec![member_key],
        ));

        let a = serde_json::to_string(&vote_plan).unwrap();
        assert_eq!(vote_plan, serde_json::from_str(&a).unwrap());
    }
}
