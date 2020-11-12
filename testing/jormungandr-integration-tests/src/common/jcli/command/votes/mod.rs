#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

pub mod committee;
mod crs;
mod encrypting_vote_key;
mod tally;

pub use committee::CommitteeCommand;
pub use crs::CrsCommand;
pub use encrypting_vote_key::EncryptingVoteKeyCommand;
pub use tally::TallyCommand;

#[derive(Debug)]
pub struct VotesCommand {
    command: Command,
}

impl VotesCommand {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn crs(mut self) -> CrsCommand {
        self.command.arg("crs");
        CrsCommand::new(self.command)
    }

    pub fn committee(mut self) -> CommitteeCommand {
        self.command.arg("committee");
        CommitteeCommand::new(self.command)
    }

    pub fn encrypting_vote_key<S: Into<String>, P: AsRef<Path>>(
        mut self,
        member_key: S,
        output_file: P,
    ) -> Self {
        self.command
            .arg("encrypting-key")
            .arg(&output_file.as_ref())
            .arg("--keys")
            .arg(member_key.into());
        self
    }

    pub fn tally(mut self) -> TallyCommand {
        self.command.arg("tally");
        TallyCommand::new(self.command)
    }

    pub fn build(self) -> Command {
        self.command
    }
}
