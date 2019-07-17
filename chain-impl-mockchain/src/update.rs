//use crate::certificate::{verify_certificate, HasPublicKeys, SignatureRaw};
use crate::date::BlockDate;
use crate::fragment::config::ConfigParams;
use crate::leadership::{bft, genesis::ActiveSlotsCoeffError};
use crate::setting::Settings;
use chain_core::mempack::{ReadBuf, ReadError, Readable};
use chain_core::property;
use chain_crypto::Verification;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
    BadBftSlotsRatio(crate::milli::Milli),
    BadConsensusGenesisPraosActiveSlotsCoeff(ActiveSlotsCoeffError),
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
            Error::BadBftSlotsRatio(m) => {
                write!(f, "Cannot set BFT slots ratio to invalid value {}", m)
            }
            Error::BadConsensusGenesisPraosActiveSlotsCoeff(err) => write!(
                f,
                "Cannot set consensus genesis praos active slots coefficient: {}",
                err
            ),
        }
    }
}

impl std::error::Error for Error {}

impl From<ActiveSlotsCoeffError> for Error {
    fn from(err: ActiveSlotsCoeffError) -> Self {
        Error::BadConsensusGenesisPraosActiveSlotsCoeff(err)
    }
}

pub type UpdateProposalId = crate::fragment::FragmentId;
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
}

impl SignedUpdateProposal {
    pub fn verify(&self) -> Verification {
        Verification::Success
    }
}

impl property::Serialize for SignedUpdateProposal {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.proposal.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateProposal {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(Self {
            proposal: Readable::read(buf)?,
        })
    }
}

// A positive vote for a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateVote {
    pub proposal_id: UpdateProposalId,
    pub voter_id: UpdateVoterId,
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
}

impl SignedUpdateVote {
    pub fn verify(&self) -> Verification {
        Verification::Success
    }
}

impl property::Serialize for SignedUpdateVote {
    type Error = std::io::Error;
    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        use chain_core::packer::*;
        let mut codec = Codec::new(writer);
        self.vote.serialize(&mut codec)?;
        Ok(())
    }
}

impl Readable for SignedUpdateVote {
    fn read<'a>(buf: &mut ReadBuf<'a>) -> Result<Self, ReadError> {
        Ok(SignedUpdateVote {
            vote: Readable::read(buf)?,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;

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
            }
        }
    }

    use crate::{
        block::{Block, BlockBuilder, HeaderHash},
        ledger::ledger::Ledger,
        testing::arbitrary::update_proposal::UpdateProposalData,
        testing::ledger as mock_ledger,
        update::{
            SignedUpdateProposal, SignedUpdateVote, UpdateProposal, UpdateProposalWithProposer,
            UpdateVote,
        },
    };
    use chain_core::property::ChainLength;
    use chain_crypto::{Ed25519, SecretKey};

    #[quickcheck]
    pub fn ledger_adopt_settiings_from_update_proposal(
        update_proposal_data: UpdateProposalData,
    ) -> TestResult {
        let config = mock_ledger::ConfigBuilder::new()
            .with_leaders(&update_proposal_data.leaders_ids())
            .build();

        let (block0_hash, mut ledger) =
            mock_ledger::create_initial_fake_ledger(&[], config).unwrap();

        // apply proposal
        let date = ledger.date();
        ledger = ledger
            .apply_update_proposal(
                update_proposal_data.proposal_id,
                &update_proposal_data.proposal,
                date,
            )
            .unwrap();

        // apply votes
        for vote in update_proposal_data.votes.iter() {
            ledger = ledger.apply_update_vote(&vote).unwrap();
        }

        // trigger proposal process (build block)
        let block = build_block(
            &ledger,
            block0_hash,
            date.next_epoch(),
            &update_proposal_data.block_signing_key,
        );
        let header_meta = block.header.to_content_eval_context();
        ledger = ledger
            .apply_block(
                &ledger.get_ledger_parameters(),
                block.contents.iter(),
                &header_meta,
            )
            .unwrap();

        // assert
        let actual_params = ledger.settings.to_config_params();
        let expected_params = update_proposal_data.proposal_settings();

        let mut all_settings_equal = true;
        for expected_param in expected_params.iter() {
            if !actual_params.iter().any(|x| x == expected_param) {
                all_settings_equal = false;
                break;
            }
        }

        match all_settings_equal {
            false => TestResult::error(format!("Error: proposed update reached required votes, but proposal was NOT updated, Expected: {:?} vs Actual: {:?}",
                                expected_params,actual_params)),
            true => TestResult::passed(),
        }
    }

    fn build_block(
        ledger: &Ledger,
        block0_hash: HeaderHash,
        date: BlockDate,
        block_signing_key: &SecretKey<Ed25519>,
    ) -> Block {
        let mut block_builder = BlockBuilder::new();
        block_builder.chain_length(ledger.chain_length.next());
        block_builder.parent(block0_hash);
        block_builder.date(date.next_epoch());
        block_builder.make_bft_block(block_signing_key)
    }

}
