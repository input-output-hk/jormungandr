use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate;
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

/// vote plan proposal related information
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Proposal {
    /// proposal id
    #[structopt(long = "proposal-id")]
    pub external_proposal_id: certificate::ExternalProposalId,
    #[structopt(skip = 0b0011)]
    pub options: u8,
}

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

    /// vommittee end block date
    ///
    /// It should be provided in the format of `epoch.slot_id`, ex: 0.0
    #[structopt(long = "committee-end")]
    pub committee_end: BlockDate,

    #[structopt(flatten)]
    pub proposal: Proposal,

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
        proposals.push(certificate::Proposal::new(
            self.proposal.external_proposal_id,
            certificate::VoteOptions::new_length(self.proposal.options),
        ));
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
