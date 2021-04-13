use crate::jcli_lib::certificate::{write_cert, Error};
use chain_impl_mockchain::certificate;
use chain_impl_mockchain::certificate::{Certificate, VotePlanId};
use std::path::PathBuf;
use structopt::StructOpt;

/// create an encrypted vote tally certificate
///
/// voteplan id needs to be provided
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct EncryptedVoteTally {
    /// vote plan id
    ///
    /// the vote plan identifier on the blockchain
    #[structopt(long = "vote-plan-id")]
    pub id: VotePlanId,

    /// write the output to the given file or print it to the standard output if not defined
    #[structopt(long = "output")]
    pub output: Option<PathBuf>,
}

impl EncryptedVoteTally {
    pub fn exec(self) -> Result<(), Error> {
        let vote_tally = certificate::EncryptedVoteTally::new(self.id);
        let cert = Certificate::EncryptedVoteTally(vote_tally);
        write_cert(self.output.as_deref(), cert.into())
    }
}
