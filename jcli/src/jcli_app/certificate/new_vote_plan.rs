use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::block::BlockDate;
use chain_impl_mockchain::certificate::{Certificate, Proposals, VotePlan};
use std::ops::Deref;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct VotePlanRegistration {
    #[structopt(short = "vs", long = "vote-start")]
    pub vote_start: BlockDate,

    #[structopt(short = "ve", long = "vote-end")]
    pub vote_end: BlockDate,

    #[structopt(short = "ce", long = "committee-end")]
    pub committee_end: BlockDate,

    // #[structopt(short = "p", long = "proposals")]
    // pub proposals: Proposals,
    #[structopt(short = "o", long = "output")]
    pub output: Option<PathBuf>,
}

impl VotePlanRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let vote_plan = VotePlan::new(
            self.vote_start,
            self.vote_end,
            self.committee_end,
            Proposals::new(),
        );
        let cert = Certificate::VotePlan(vote_plan);
        write_cert(self.output.as_ref().map(|x| x.deref()), cert.into())
    }
}
