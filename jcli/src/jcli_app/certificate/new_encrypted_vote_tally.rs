use crate::jcli_app::certificate::{write_cert, Error};
use chain_impl_mockchain::certificate::{Certificate, EncryptedVoteTally, VotePlanId};
use std::path::PathBuf;
use structopt::StructOpt;

/// create a vote tally certificate
///
/// voteplan id needs to be provided
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct EncryptedVoteTallyRegistration {
    /// vote plan id
    ///
    /// the vote plan identifier on the blockchain
    #[structopt(long = "vote-plan-id")]
    pub id: VotePlanId,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl EncryptedVoteTallyRegistration {
    pub fn exec(self) -> Result<(), Error> {
        let vote_tally = EncryptedVoteTally::new(self.id);
        let cert = Certificate::EncryptedVoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}
