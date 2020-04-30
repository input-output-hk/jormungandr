use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate;
use std::ops::Deref;
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

    // list of proposal ids to add to the vote plan certificate
    #[structopt(long = "proposals-ids")]
    pub proposals: Vec<certificate::ExternalProposalId>,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
        // check that the block dates are consecutive
        if self.vote_start > self.vote_end {
            return Err(Error::InvalidVotePlanVoteBlockDates {
                vote_start: self.vote_start.to_string(),
                vote_end: self.vote_end.to_string(),
            });
        }
        if self.vote_end > self.committee_end {
            return Err(Error::InvalidVotePlanCommitteeBlockDates {
                vote_end: self.vote_end.to_string(),
                committee_end: self.committee_end.to_string(),
            });
        }

        // build certificate
        let mut proposals = certificate::Proposals::new();
        for proposal_id in self.proposals {
            proposals.push(certificate::Proposal::new(
                proposal_id,
                certificate::VoteOptions::new_length(0b0011),
            ));
        }
        let vote_plan = certificate::VotePlan::new(
            self.vote_start,
            self.vote_end,
            self.committee_end,
            certificate::Proposals::new(),
        );
        let cert = certificate::Certificate::VotePlan(vote_plan);
        write_cert(self.output.as_ref().map(|x| x.deref()), cert.into())
    }
}
