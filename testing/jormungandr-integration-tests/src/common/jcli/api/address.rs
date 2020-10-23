use crate::common::jcli::command::{AddressCommand, GenesisCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::ChildPath;
use chain_addr::Discrimination;
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::prelude::ProcessOutput;
use std::str::FromStr;
use std::{collections::BTreeMap, path::Path};
pub struct JCliAddress {
    address_command: AddressCommand,
}

impl JCliAddress {
    pub fn new(address_command: AddressCommand) -> Self {
        Self { address_command }
    }

    pub fn info<S: Into<String>>(self, public_key: S) -> BTreeMap<String, String> {
        self.address_command
            .info()
            .address(public_key.into())
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_node_yaml()
    }

    pub fn account<S: Into<String>>(
        self,
        prefix: Option<S>,
        public_key: S,
        discrimination: Discrimination,
    ) -> String {
        let mut address_command = self.address_command.account();

        if let Some(prefix) = prefix {
            address_command = address_command.prefix(prefix.into());
        }

        if discrimination == Discrimination::Test {
            address_command = address_command.test_discrimination();
        }

        address_command
            .public_key(public_key)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn account_expect_fail<S: Into<String>>(
        self,
        prefix: Option<S>,
        public_key: S,
        expected_msg: &str,
    ) {
        let mut address_command = self.address_command.account();

        if let Some(prefix) = prefix {
            address_command = address_command.prefix(prefix.into());
        }

        if discrimination == Discrimination::Test {
            address_command = address_command.test_discrimination();
        }

        address_command
            .public_key(public_key)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn info_expect_fail<S: Into<String>>(self, public_key: S, expected_msg: &str) {
        self.address_command
            .info()
            .address(public_key.into())
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn single<S: Into<String>>(
        self,
        prefix: Option<S>,
        public_key: S,
        discrimination: Discrimination,
    ) -> String {
        let mut address_command = self.address_command.single();

        if let Some(prefix) = prefix {
            address_command = address_command.prefix(prefix.into());
        }

        if discrimination == Discrimination::Test {
            address_command = address_command.test_discrimination();
        }

        address_command
            .public_key(public_key)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn delegation<S: Into<String>, P: Into<String>>(
        mut self,
        public_key: S,
        delegation_key: P,
        discrimination: Discrimination,
    ) -> String {
        let mut address_command = self.address_command.single();

        if let Some(prefix) = prefix {
            address_command = address_command.prefix(prefix.into());
        }

        if discrimination == Discrimination::Test {
            address_command = address_command.test_discrimination();
        }

        address_command
            .public_key(public_key)
            .delegation_key(delegation_key)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }
}
