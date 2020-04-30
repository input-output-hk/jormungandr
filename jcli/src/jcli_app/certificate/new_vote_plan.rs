use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate;
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct Proposal {
    #[structopt(long = "proposal-id")]
    pub external_proposal_id: certificate::ExternalProposalId,
    #[structopt(default_value = "0b0011")]
    pub options: u8,
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotePlanRegistration {
    #[structopt(long = "vote-start")]
    pub vote_start: BlockDate,

    #[structopt(long = "vote-end")]
    pub vote_end: BlockDate,

    #[structopt(long = "committee-end")]
    pub committee_end: BlockDate,

    #[structopt(subcommand)]
    pub proposal: Proposal,

    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
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
