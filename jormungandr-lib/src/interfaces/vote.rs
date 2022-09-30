use crate::{
    crypto::hash::Hash,
    interfaces::{blockdate::BlockDate, mint_token::TokenIdentifier, value::ValueDef},
};
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::{
    certificate::{self, ExternalProposalId, Proposal, Proposals, VoteAction},
    ledger::governance::{ParametersGovernanceAction, TreasuryGovernanceAction},
    value::Value,
    vote::{self, Choice, Options, Weight},
};
use chain_vote::MemberPublicKey;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::TryInto,
    fmt,
    ops::Range,
    str::{self, FromStr},
};

/// Serializable wrapper for the payload type enum.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct VotePrivacy(#[serde(with = "PayloadTypeDef")] pub vote::PayloadType);

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(remote = "vote::PayloadType", rename_all = "snake_case")]
enum PayloadTypeDef {
    Public,
    Private,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid vote privacy, expected \"public\" or \"private\".")]
pub struct VotePrivacyFromStrError;

impl FromStr for VotePrivacy {
    type Err = VotePrivacyFromStrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(VotePrivacy(vote::PayloadType::Public)),
            "private" => Ok(VotePrivacy(vote::PayloadType::Private)),
            _ => Err(VotePrivacyFromStrError),
        }
    }
}

impl fmt::Display for VotePrivacy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self.0 {
            vote::PayloadType::Public => "public",
            vote::PayloadType::Private => "private",
        };
        s.fmt(f)
    }
}

impl From<vote::PayloadType> for VotePrivacy {
    fn from(src: vote::PayloadType) -> Self {
        VotePrivacy(src)
    }
}

impl From<VotePrivacy> for vote::PayloadType {
    fn from(src: VotePrivacy) -> Self {
        src.0
    }
}

struct SerdeMemberPublicKey(chain_vote::MemberPublicKey);

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
                    MemberPublicKey::BECH32_HRP
                )
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(SerdeMemberPublicKey(
                    MemberPublicKey::try_from_bech32_str(value).map_err(|err| {
                        serde::de::Error::custom(format!(
                            "Invalid public key with bech32 representation {}, Error {}",
                            &value, err
                        ))
                    })?,
                ))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v)
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
            serializer.serialize_str(&self.0.to_bech32_str())
        } else {
            serializer.serialize_bytes(&self.0.to_bytes())
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, Eq, PartialEq)]
pub struct VotePlan {
    pub payload_type: VotePrivacy,
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    #[serde(with = "serde_proposals")]
    pub proposals: Proposals,
    #[serde(with = "serde_committee_member_public_keys", default = "Vec::new")]
    pub committee_member_public_keys: Vec<chain_vote::MemberPublicKey>,
    pub voting_token: TokenIdentifier,
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

impl From<certificate::VotePlan> for VotePlan {
    fn from(vp: certificate::VotePlan) -> Self {
        VotePlan {
            vote_start: vp.vote_start().into(),
            vote_end: vp.vote_end().into(),
            committee_end: vp.committee_end().into(),
            proposals: vp.proposals().clone(),
            payload_type: vp.payload_type().into(),
            committee_member_public_keys: vp.committee_public_keys().to_vec(),
            voting_token: vp.voting_token().clone().into(),
        }
    }
}

impl From<VotePlan> for certificate::VotePlan {
    fn from(vpd: VotePlan) -> Self {
        certificate::VotePlan::new(
            vpd.vote_start.into(),
            vpd.vote_end.into(),
            vpd.committee_end.into(),
            vpd.proposals,
            vpd.payload_type.into(),
            vpd.committee_member_public_keys,
            vpd.voting_token.into(),
        )
    }
}

pub mod serde_committee_member_public_keys {
    use crate::interfaces::vote::SerdeMemberPublicKey;
    use serde::{
        de::{SeqAccess, Visitor},
        ser::SerializeSeq,
        Deserializer, Serializer,
    };

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

pub mod serde_external_proposal_id {
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

pub mod serde_choices {
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

pub mod serde_proposals {
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

pub type VotePlanId = Hash;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct AccountVotes {
    pub vote_plan_id: VotePlanId,
    pub votes: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct VotePlanStatus {
    pub id: VotePlanId,
    #[serde(with = "PayloadTypeDef")]
    pub payload: vote::PayloadType,
    pub vote_start: BlockDate,
    pub vote_end: BlockDate,
    pub committee_end: BlockDate,
    #[serde(with = "serde_committee_member_public_keys")]
    pub committee_member_keys: Vec<MemberPublicKey>,
    pub proposals: Vec<VoteProposalStatus>,
    pub voting_token: TokenIdentifier,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Tally {
    Public { result: TallyResult },
    Private { state: PrivateTallyState },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TallyResult {
    pub results: Vec<u64>,
    pub options: Range<u8>,
}

impl TallyResult {
    pub fn results(&self) -> Vec<u64> {
        self.results.clone()
    }

    pub fn merge(&self, other: &Self) -> Self {
        assert_eq!(self.options, other.options);

        Self {
            results: self
                .results
                .iter()
                .zip(other.results().iter())
                .map(|(l, r)| l + r)
                .collect(),
            options: self.options.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedTally(#[serde(with = "serde_base64_bytes")] Vec<u8>);

impl EncryptedTally {
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for EncryptedTally {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub mod serde_base64_bytes {
    use serde::{
        de::{Error, Visitor},
        Deserializer, Serializer,
    };

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
    Encrypted { encrypted_tally: EncryptedTally },
    Decrypted { result: TallyResult },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VotePayload {
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VoteProposalStatus {
    pub index: u8,
    pub proposal_id: Hash,
    pub options: Range<u8>,
    pub tally: Tally,
    pub votes_cast: usize,
}

impl From<vote::Payload> for VotePayload {
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

impl VotePayload {
    pub fn choice(&self) -> Option<u8> {
        match self {
            VotePayload::Public { choice } => Some(*choice),
            VotePayload::Private { .. } => None,
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
            results: this.votes.to_vec(),
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
                    vote::PrivateTallyState::Encrypted { encrypted_tally } => {
                        PrivateTallyState::Encrypted {
                            encrypted_tally: EncryptedTally(encrypted_tally.to_bytes()),
                        }
                    }
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
                    PrivateTallyState::Encrypted { encrypted_tally } => {
                        vote::PrivateTallyState::Encrypted {
                            encrypted_tally: encrypted_tally.into(),
                        }
                    }
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
            tally: this.tally.into(),
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
            tally: vote_proposal_status.tally.into(),
            votes: Default::default(),
        }
    }
}

impl From<vote::VotePlanStatus> for VotePlanStatus {
    fn from(this: vote::VotePlanStatus) -> Self {
        Self {
            id: this.id.into(),
            vote_start: this.vote_start.into(),
            vote_end: this.vote_end.into(),
            committee_end: this.committee_end.into(),
            payload: this.payload,
            committee_member_keys: this.committee_public_keys,
            proposals: this.proposals.into_iter().map(|p| p.into()).collect(),
            voting_token: this.voting_token.into(),
        }
    }
}

impl From<VotePlanStatus> for vote::VotePlanStatus {
    fn from(vote_plan_status: VotePlanStatus) -> vote::VotePlanStatus {
        vote::VotePlanStatus {
            id: vote_plan_status.id.into(),
            vote_start: vote_plan_status.vote_start.into(),
            vote_end: vote_plan_status.vote_end.into(),
            committee_end: vote_plan_status.committee_end.into(),
            payload: vote_plan_status.payload,
            committee_public_keys: vote_plan_status.committee_member_keys,
            proposals: vote_plan_status
                .proposals
                .into_iter()
                .map(|p| p.into())
                .collect(),
            voting_token: vote_plan_status.voting_token.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::interfaces::vote::{serde_committee_member_public_keys, SerdeMemberPublicKey};
    use chain_impl_mockchain::{block::BlockDate, certificate, tokens::identifier};
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn test_deserialize_member_public_keys() {
        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let crs = chain_vote::Crs::from_hash("Dummy shared string".as_bytes());
        let comm_key = chain_vote::MemberCommunicationKey::new(&mut rng);

        let member_key =
            chain_vote::MemberState::new(&mut rng, 1, &crs, &[comm_key.to_public()], 0);
        let pk = member_key.public_key();
        let pks = vec![pk.to_bech32_str()];
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
        let mut rng = rand_chacha::ChaChaRng::from_entropy();
        let crs = chain_vote::Crs::from_hash("Dummy shared string".as_bytes());
        let comm_key = chain_vote::MemberCommunicationKey::new(&mut rng);

        let member_key =
            chain_vote::MemberState::new(&mut rng, 1, &crs, &[comm_key.to_public()], 0)
                .public_key();
        let start = "42.12".parse::<BlockDate>().unwrap();
        let end = "42.13".parse::<BlockDate>().unwrap();
        let tally = "42.14".parse::<BlockDate>().unwrap();
        let id = ExternalProposalId::from([0; 32]);
        let prop = Proposal::new(id, Options::new_length(1).unwrap(), VoteAction::OffChain);
        let mut proposals = Proposals::new();
        let _ = proposals.push(prop);
        let voting_token = identifier::TokenIdentifier::from_str(
            "00000000000000000000000000000000000000000000000000000000.00000000",
        )
        .unwrap();
        let vote_plan: VotePlan = certificate::VotePlan::new(
            start,
            end,
            tally,
            proposals,
            vote::PayloadType::Private,
            vec![member_key],
            voting_token,
        )
        .into();

        let a = serde_json::to_string(&vote_plan).unwrap();
        assert_eq!(vote_plan, serde_json::from_str(&a).unwrap());
    }
}
