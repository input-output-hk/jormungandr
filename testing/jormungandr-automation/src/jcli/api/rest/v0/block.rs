use crate::jcli::command::rest::v0::BlockCommand;
use assert_cmd::assert::OutputAssertExt;
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::prelude::ProcessOutput;

pub struct Block {
    block_command: BlockCommand,
}

impl Block {
    pub fn new(block_command: BlockCommand) -> Self {
        Self { block_command }
    }

    pub fn get<P: Into<String>, S: Into<String>>(self, block_id: P, host: S) -> String {
        self.block_command
            .get(block_id, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }

    pub fn get_expect_fail<P: Into<String>, S: Into<String>>(
        self,
        block_id: P,
        host: S,
        expected_msg: &str,
    ) {
        self.block_command
            .get(block_id, host)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn next<P: Into<String>, S: Into<String>>(
        self,
        block_id: P,
        limit: u32,
        host: S,
    ) -> Vec<Hash> {
        let content = self
            .block_command
            .next(block_id, limit, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_multi_line();

        content
            .iter()
            .map(|s| Hash::from_hex(s).unwrap())
            .collect::<Vec<Hash>>()
    }

    pub fn next_expect_fail<P: Into<String>, S: Into<String>>(
        self,
        block_id: P,
        limit: u32,
        host: S,
        expected_msg: &str,
    ) {
        self.block_command
            .next(block_id.into(), limit, host.into())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }
}
