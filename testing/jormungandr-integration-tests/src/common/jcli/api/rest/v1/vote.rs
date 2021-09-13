use crate::common::jcli::command::rest::v1::VoteCommand;
use assert_cmd::assert::OutputAssertExt;
use jortestkit::prelude::ProcessOutput;

pub struct Vote {
    vote_command: VoteCommand,
}

impl Vote {
    pub fn new(vote_command: VoteCommand) -> Self {
        Self { vote_command }
    }

    pub fn account_votes(
        self,
        account: impl Into<String>,
        voteplan: impl Into<String>,
        host: impl Into<String>,
    ) -> Vec<u8> {
        let content = self
            .vote_command
            .account_votes(account, voteplan, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("JCLI returned malformed proposals")
    }
}
