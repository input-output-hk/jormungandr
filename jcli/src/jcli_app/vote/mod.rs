mod committee_member_key;
mod encrypting_vote_key;

use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Vote {
    /// Build an encryption vote key
    NewEncryptingVoteKey(encrypting_vote_key::BuildEncryptingVoteKey),
}

impl Vote {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Vote::NewEncryptingVoteKey(cmd) => cmd.exec(),
        }
    }
}
