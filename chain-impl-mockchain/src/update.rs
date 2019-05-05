use crate::certificate::{verify_certificate, HasPublicKeys, SignatureRaw};
use crate::date::BlockDate;
use crate::{leadership::bft, message::config::ConfigParams, setting::Settings};
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::{Ed25519Extended, PublicKey, SecretKey, Verification};
use std::collections::{BTreeMap, HashSet};
use std::iter;

#[derive(Clone, Debug)]
pub struct UpdateState {
    // Note: we use a BTreeMap to ensure that proposals are processed
    // in a well-defined (sorted) order.
    pub proposals: BTreeMap<UpdateProposalId, UpdateProposalState>,
}

impl UpdateState {
    pub fn new() -> Self {
        UpdateState {
            proposals: BTreeMap::new(),
        }
    }

    pub fn apply_proposal(
        mut self,
        proposal_id: UpdateProposalId,
        proposal: &SignedUpdateProposal,
        settings: &Settings,
        cur_date: BlockDate,
    ) -> Result<Self, Error> {
        let proposer_id = &proposal.proposal.proposer_id;

        if proposal.verify() == Verification::Failed {
            return Err(Error::BadProposalSignature(
                proposal_id,
                proposer_id.clone(),
            ));
        }

        if !settings.bft_leaders.contains(proposer_id) {
            return Err(Error::BadProposer(proposal_id, proposer_id.clone()));
        }

        let proposal = &proposal.proposal.proposal;

        if let Some(_) = self.proposals.get_mut(&proposal_id) {
            Err(Error::DuplicateProposal(proposal_id))
        } else {
            self.proposals.insert(
                proposal_id,
                UpdateProposalState {
                    proposal: proposal.clone(),
                    proposal_date: cur_date,
                    votes: HashSet::new(),
                },
            );
            Ok(self)
        }
    }

    pub fn apply_vote(
        mut self,
        vote: &SignedUpdateVote,
        settings: &Settings,
    ) -> Result<Self, Error> {
        if vote.verify() == Verification::Failed {
            return Err(Error::BadVoteSignature(
                vote.vote.proposal_id.clone(),
                vote.vote.voter_id.clone(),
            ));
        }

        let vote = &vote.vote;

        if !settings.bft_leaders.contains(&vote.voter_id) {
            return Err(Error::BadVoter(
                vote.proposal_id.clone(),
                vote.voter_id.clone(),
            ));
        }

        if let Some(proposal) = self.proposals.get_mut(&vote.proposal_id) {
            if !proposal.votes.insert(vote.voter_id.clone()) {
                return Err(Error::DuplicateVote(
                    vote.proposal_id.clone(),
                    vote.voter_id.clone(),
                ));
            }

            Ok(self)
        } else {
            Err(Error::VoteForMissingProposal(vote.proposal_id.clone()))
        }
    }

    pub fn process_proposals(
        mut self,
        mut settings: Settings,
        prev_date: BlockDate,
        new_date: BlockDate,
    ) -> Result<(Self, Settings), Error> {
        let mut expired_ids = vec![];

        assert!(prev_date < new_date);

        // If we entered a new epoch, then delete expired update
        // proposals and apply accepted update proposals.
        if prev_date.epoch < new_date.epoch {
            for (proposal_id, proposal_state) in &self.proposals {
                // If a majority of BFT leaders voted for the
                // proposal, then apply it. FIXME: multiple proposals
                // might become accepted at the same time, in which
                // case they're currently applied in order of proposal
                // ID. FIXME: delay the effectuation of the proposal
                // for some number of epochs.
                if proposal_state.votes.len() > settings.bft_leaders.len() / 2 {
                    settings = settings.apply(&proposal_state.proposal.changes)?;
                    expired_ids.push(proposal_id.clone());
                } else if proposal_state.proposal_date.epoch + settings.proposal_expiration
                    > new_date.epoch
                {
                    expired_ids.push(proposal_id.clone());
                }
            }

            for proposal_id in expired_ids {
                self.proposals.remove(&proposal_id);
            }
        }

        Ok((self, settings))
    }
}

#[derive(Clone, Debug)]
pub struct UpdateProposalState {
    pub proposal: UpdateProposal,
    pub proposal_date: BlockDate,
    pub votes: HashSet<UpdateVoterId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /*
    InvalidCurrentBlockId(Hash, Hash),
    UpdateIsInvalid,
     */
    BadProposalSignature(UpdateProposalId, UpdateVoterId),
    BadProposer(UpdateProposalId, UpdateVoterId),
    DuplicateProposal(UpdateProposalId),
    VoteForMissingProposal(UpdateProposalId),
    BadVoteSignature(UpdateProposalId, UpdateVoterId),
    BadVoter(UpdateProposalId, UpdateVoterId),
    DuplicateVote(UpdateProposalId, UpdateVoterId),
    ReadOnlySetting,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            /*
            Error::InvalidCurrentBlockId(current_one, update_one) => {
                write!(f, "Cannot apply Setting Update. Update needs to be applied to from block {:?} but received {:?}", update_one, current_one)
            }
            Error::UpdateIsInvalid => write!(
                f,
                "Update does not apply to current state"
            ),
             */
            Error::BadProposalSignature(proposal_id, proposer_id) => write!(
                f,
                "Proposal {} from {:?} has an incorrect signature",
                proposal_id, proposer_id
            ),
            Error::BadProposer(proposal_id, proposer_id) => write!(
                f,
                "Proposer {:?} for proposal {} is not a BFT leader",
                proposer_id, proposal_id
            ),
            Error::DuplicateProposal(proposal_id) => {
                write!(f, "Received a duplicate proposal {}", proposal_id)
            }
            Error::VoteForMissingProposal(proposal_id) => write!(
                f,
                "Received a vote for a non-existent proposal {}",
                proposal_id
            ),
            Error::BadVoteSignature(proposal_id, voter_id) => write!(
                f,
                "Vote from {:?} for proposal {} has an incorrect signature",
                voter_id, proposal_id
            ),
            Error::BadVoter(proposal_id, voter_id) => write!(
                f,
                "Voter {:?} for proposal {} is not a BFT leader",
                voter_id, proposal_id
            ),
            Error::DuplicateVote(proposal_id, voter_id) => write!(
                f,
                "Received a duplicate vote from {:?} for proposal {}",
                voter_id, proposal_id
            ),
            Error::ReadOnlySetting => write!(
                f,
                "Received a proposal to modify a chain parameter that can only be set in block 0"
            ),
        }
    }
}
impl std::error::Error for Error {}

pub type UpdateProposalId = crate::message::MessageId;
pub type UpdateVoterId = bft::LeaderId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateProposal {
    pub changes: ConfigParams,
}

impl UpdateProposal {
    pub fn new() -> Self {
        UpdateProposal {
            changes: ConfigParams::new(),
        }
    }
}

impl property::Serialize for UpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        self.changes.serialize(writer)?;
        Ok(())
    }
}

impl Readable for UpdateProposal {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(Self {
            changes: ConfigParams::read(buf)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct UpdateProposalWithProposer {
    pub proposal: UpdateProposal,
    pub proposer_id: UpdateVoterId,
}

impl<'a> HasPublicKeys<'a> for &'a UpdateProposalWithProposer {
    type PublicKeys = iter::Once<&'a PublicKey<Ed25519Extended>>;
    fn public_keys(self) -> Self::PublicKeys {
        std::iter::once(&self.proposer_id.0)
    }
}

impl property::Serialize for UpdateProposalWithProposer {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal.serialize(&mut codec)?;
        self.proposer_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for UpdateProposalWithProposer {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(Self {
            proposal: Readable::read(buf)?,
            proposer_id: Readable::read(buf)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SignedUpdateProposal {
    pub proposal: UpdateProposalWithProposer,
    pub signature: SignatureRaw,
}

impl UpdateProposal {
    pub fn make_certificate(
        &self,
        proposer_private_key: &SecretKey<Ed25519Extended>,
    ) -> SignatureRaw {
        use crate::key::make_signature;
        SignatureRaw(
            make_signature(proposer_private_key, &self)
                .as_ref()
                .to_vec(),
        )
    }
}

impl SignedUpdateProposal {
    pub fn verify(&self) -> Verification {
        verify_certificate(&self.proposal, &vec![self.signature.clone()])
    }
}

impl property::Serialize for SignedUpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal.serialize(&mut codec)?;
        self.signature.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateProposal {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(Self {
            proposal: Readable::read(buf)?,
            signature: Readable::read(buf)?,
        })
    }
}

// A positive vote for a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateVote {
    pub proposal_id: UpdateProposalId,
    pub voter_id: UpdateVoterId,
}

impl<'a> HasPublicKeys<'a> for &'a UpdateVote {
    type PublicKeys = iter::Once<&'a PublicKey<Ed25519Extended>>;
    fn public_keys(self) -> Self::PublicKeys {
        std::iter::once(&self.voter_id.0)
    }
}

impl property::Serialize for UpdateVote {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal_id.serialize(&mut codec)?;
        self.voter_id.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for UpdateVote {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(UpdateVote {
            proposal_id: Readable::read(buf)?,
            voter_id: Readable::read(buf)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct SignedUpdateVote {
    pub vote: UpdateVote,
    pub signature: SignatureRaw,
}

impl UpdateVote {
    pub fn make_certificate(&self, voter_private_key: &SecretKey<Ed25519Extended>) -> SignatureRaw {
        use crate::key::make_signature;
        SignatureRaw(make_signature(voter_private_key, &self).as_ref().to_vec())
    }
}

impl SignedUpdateVote {
    pub fn verify(&self) -> Verification {
        verify_certificate(&self.vote, &vec![self.signature.clone()])
    }
}

impl property::Serialize for SignedUpdateVote {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.vote.serialize(&mut codec)?;
        self.signature.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateVote {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(SignedUpdateVote {
            vote: Readable::read(buf)?,
            signature: Readable::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for UpdateProposal {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut changes = ConfigParams::new();
            for _ in 0..u8::arbitrary(g) % 10 {
                changes.push(Arbitrary::arbitrary(g));
            }
            Self { changes }
        }
    }

    impl Arbitrary for UpdateProposalWithProposer {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                proposal: Arbitrary::arbitrary(g),
                proposer_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedUpdateProposal {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                proposal: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for UpdateVote {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                proposal_id: Arbitrary::arbitrary(g),
                voter_id: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for SignedUpdateVote {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self {
                vote: Arbitrary::arbitrary(g),
                signature: Arbitrary::arbitrary(g),
            }
        }
    }
}
