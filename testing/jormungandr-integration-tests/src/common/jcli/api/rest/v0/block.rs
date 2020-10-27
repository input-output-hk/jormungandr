use crate::common::jcli::command::rest::v0::{BlockCommand, MessageCommand};
use crate::common::jcli::command::{AddressCommand, GenesisCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::FileWriteStr;
use assert_fs::{fixture::ChildPath, NamedTempFile};
use chain_addr::Discrimination;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::{crypto::hash::Hash, interfaces::FragmentLog};
use jormungandr_testing_utils::testing::process::ProcessOutput as _;
use jortestkit::prelude::ProcessOutput;
use std::str::FromStr;
use std::{collections::BTreeMap, path::Path};

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
        mut self,
        block_id: P,
        limit: u32,
        host: S,
    ) -> Hash {
        let content = self
            .block_command
            .next(block_id, limit, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line();
        Hash::from_hex(&content).unwrap()
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
