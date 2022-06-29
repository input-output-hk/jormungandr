use crate::jcli::command::GenesisCommand;
use assert_cmd::assert::OutputAssertExt;
use assert_fs::{assert::PathAssert, fixture::ChildPath};
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::prelude::ProcessOutput;
use std::{path::Path, str::FromStr};
pub struct Genesis {
    genesis_command: GenesisCommand,
}

impl Genesis {
    pub fn new(genesis_command: GenesisCommand) -> Self {
        Self { genesis_command }
    }

    pub fn decode<P: AsRef<Path>>(self, input: P, output: &ChildPath) {
        self.genesis_command
            .decode()
            .input(input)
            .output(output.path())
            .build()
            .assert()
            .success();
        output.assert(jortestkit::prelude::file_exists_and_not_empty());
    }

    pub fn encode<P: AsRef<Path>>(self, input: P, output: &ChildPath) {
        self.genesis_command
            .encode()
            .input(input)
            .output(output.path())
            .build()
            .assert()
            .success();

        output.assert(jortestkit::prelude::file_exists_and_not_empty());
    }

    pub fn encode_expect_fail<P: AsRef<Path>>(self, input: P, expected_msg: &str) {
        self.genesis_command
            .encode()
            .input(input)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn hash<P: AsRef<Path>>(self, input: P) -> Hash {
        let hash = self
            .genesis_command
            .hash()
            .input(input)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line();

        Hash::from_str(&hash).unwrap()
    }

    pub fn hash_expect_fail<P: AsRef<Path>>(self, input: P, expected_msg: &str) {
        self.genesis_command
            .hash()
            .input(input)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn init(self) -> String {
        self.genesis_command
            .init()
            .assert()
            .success()
            .get_output()
            .as_lossy_string()
    }
}
