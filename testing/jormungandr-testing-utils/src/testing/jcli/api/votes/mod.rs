use crate::testing::jcli::command::VotesCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{assert::PathAssert, NamedTempFile};

pub mod committee;
mod crs;
mod tally;

pub use committee::Committee;
pub use crs::Crs;
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
}
