use crate::testing::jcli::command::VotesCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{assert::PathAssert, NamedTempFile};
use std::path::Path;

pub mod committee;
mod crs;
mod tally;

pub use committee::Committee;
pub use crs::Crs;
use jortestkit::prelude::ProcessOutput;
pub use tally::Tally;

pub struct Votes {
    votes_command: VotesCommand,
}

impl Votes {
    pub fn new(votes_command: VotesCommand) -> Self {
        Self { votes_command }
    }

    pub fn committee(self) -> Committee {
        Committee::new(self.votes_command.committee())
    }

    pub fn crs(self) -> Crs {
        Crs::new(self.votes_command.crs())
    }

    pub fn election_public_key<S: Into<String>>(self, member_key: S) -> String {
        let output_file = NamedTempFile::new("election_public_key.tmp").unwrap();
        self.votes_command
            .election_public_key(member_key, output_file.path())
            .build()
            .assert()
            .success();

        output_file.assert(jortestkit::prelude::file_exists_and_not_empty());
        jortestkit::prelude::read_file(output_file.path())
    }

    pub fn tally(self) -> Tally {
        Tally::new(self.votes_command.tally())
    }

    pub fn update_proposal<P: AsRef<Path>, Q: AsRef<Path>>(self, config: P, secret: Q) -> String {
        self.votes_command
            .update_proposal(config, secret)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn update_vote<R: Into<String>, P: AsRef<Path>>(self, proposal_id: R, secret: P) -> String {
        self.votes_command
            .update_vote(proposal_id, secret)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }
}
