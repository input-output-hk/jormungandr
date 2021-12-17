mod all;
mod votes_only;

use crate::mjolnir_lib::MjolnirError;
use structopt::StructOpt;
pub use votes_only::VotesOnly;

#[derive(StructOpt, Debug)]
pub enum Adversary {
    VotesOnly(votes_only::VotesOnly),
    All(all::AllAdversary),
}

impl Adversary {
    pub fn exec(&self) -> Result<(), MjolnirError> {
        match self {
            Adversary::VotesOnly(votes_only_command) => votes_only_command.exec(),
            Adversary::All(all_command) => all_command.exec(),
        }
    }
}
