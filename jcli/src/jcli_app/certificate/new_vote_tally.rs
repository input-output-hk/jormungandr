use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::certificate::{Certificate, VotePlanId, VoteTally};
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote tally certificate
///
/// voteplan id needs to be provided
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VoteTallyRegistration {
    /// vote plan id
    ///
    /// the vote plan identifier on the blockchain
    #[structopt(long = "vote-plan-id")]
    pub id: VotePlanId,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl VoteTallyRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let vote_tally = VoteTally::new_public(self.id);
        let cert = Certificate::VoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}
