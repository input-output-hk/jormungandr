#![allow(dead_code)]

use std::{path::Path, process::Command};

pub mod committee;
mod crs;
mod election_public_key;
mod tally;

pub use committee::CommitteeCommand;
pub use crs::CrsCommand;
pub use election_public_key::ElectionPublicKeyCommand;
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

    pub fn election_public_key<S: Into<String>, P: AsRef<Path>>(
        mut self,
        member_key: S,
        output_file: P,
    ) -> Self {
        self.command
            .arg("election-key")
            .arg(output_file.as_ref())
            .arg("--keys")
            .arg(member_key.into());
        self
    }

    pub fn tally(mut self) -> TallyCommand {
        self.command.arg("tally");
        TallyCommand::new(self.command)
    }

    pub fn update_proposal<P: AsRef<Path>, Q: AsRef<Path>>(mut self, config: P, secret: Q) -> Self {
        self.command
            .arg("update-proposal")
            .arg(config.as_ref())
            .arg("--secret")
            .arg(secret.as_ref());
        self
    }

    pub fn update_vote<R: Into<String>, P: AsRef<Path>>(
        mut self,
        proposal_id: R,
        secret: P,
    ) -> Self {
        self.command
            .arg("update-vote")
            .arg(proposal_id.into())
            .arg("--secret")
            .arg(secret.as_ref());
        self
    }

    pub fn build(self) -> Command {
        self.command
    }
}
