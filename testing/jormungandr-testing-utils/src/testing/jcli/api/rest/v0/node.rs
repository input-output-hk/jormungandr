use crate::testing::jcli::command::rest::v0::NodeCommand;
use assert_cmd::assert::OutputAssertExt;
use jortestkit::prelude::ProcessOutput;
use std::collections::BTreeMap;

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
