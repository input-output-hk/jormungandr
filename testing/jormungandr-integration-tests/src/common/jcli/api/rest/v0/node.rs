use crate::common::jcli::command::rest::v0::NodeCommand;
use crate::common::jcli::command::{AddressCommand, GenesisCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::ChildPath;
use chain_addr::Discrimination;
use jormungandr_lib::crypto::hash::Hash;
use jortestkit::prelude::ProcessOutput;
use std::str::FromStr;
use std::{collections::BTreeMap, path::Path};

pub struct Node {
    node_command: NodeCommand,
}

impl Node {
    pub fn new(node_command: NodeCommand) -> Self {
        Self { node_command }
    }

    pub fn stats<S: Into<String>>(self, host: S) -> BTreeMap<String, String> {
        self.node_command
            .stats(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_node_yaml()
    }
}
