use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{Certificate, ExternalProposalId, Proposal, Proposals, VoteOptions, VotePlan},
};
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote plan certificate
///
/// 3 Block dates need to be provided as well as the proposal id
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotePlanRegistration {
    /// vote start block date
    ///
    /// It should be provided in the format of `epoch.slot_id`, ex: 0.0
    #[structopt(long = "vote-start")]
    pub vote_start: BlockDate,

    /// vote end block date
    ///
    /// It should be provided in the format of `epoch.slot_id`, ex: 0.0
    #[structopt(long = "vote-end")]
    pub vote_end: BlockDate,

    /// committee end block date
    ///
    /// It should be provided in the format of `epoch.slot_id`, ex: 0.0
    #[structopt(long = "committee-end")]
    pub committee_end: BlockDate,

    /// proposal id to add to the vote plan certificate
    ///
    /// There may not be more than 255 proposals per vote plan certificate
    #[structopt(long = "proposal-id")]
    pub proposals: Vec<ExternalProposalId>,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
        // check that the block dates are consecutive
        if self.vote_start >= self.vote_end {
            return Err(Error::InvalidVotePlanVoteBlockDates {
                vote_start: self.vote_start,
                vote_end: self.vote_end,
            });
        }
        if self.vote_end >= self.committee_end {
            return Err(Error::InvalidVotePlanCommitteeBlockDates {
                vote_end: self.vote_end,
                committee_end: self.committee_end,
            });
        }
        if self.proposals.len() > Proposals::MAX_LEN {
            return Err(Error::TooManyVotePlanProposals {
                actual: self.proposals.len(),
                max: Proposals::MAX_LEN,
            });
        }

        // build certificate
        let mut proposals = Proposals::new();
        for proposal_id in self.proposals {
            let _ = proposals.push(Proposal::new(proposal_id, VoteOptions::new_length(0b0011)));
        }
        let vote_plan = VotePlan::new(
            self.vote_start,
            self.vote_end,
            self.committee_end,
            proposals,
        );
        let cert = Certificate::VotePlan(vote_plan);
        write_cert(self.output.as_deref(), cert.into())
    }
}
