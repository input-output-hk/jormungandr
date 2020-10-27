use crate::common::jcli::command::rest::v0::{MessageCommand, VoteCommand};
use crate::common::jcli::command::{AddressCommand, GenesisCommand};
use assert_cmd::assert::OutputAssertExt;
use assert_fs::assert::PathAssert;
use assert_fs::fixture::FileWriteStr;
use assert_fs::{fixture::ChildPath, NamedTempFile};
use chain_addr::Discrimination;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{CommitteeIdDef, FragmentLog},
};
use jormungandr_testing_utils::testing::process::ProcessOutput as _;
use jortestkit::prelude::ProcessOutput;
use serde_json::Value;
use std::str::FromStr;
use std::{collections::BTreeMap, path::Path};

pub struct Vote {
    vote_command: VoteCommand,
}

impl Vote {
    pub fn new(vote_command: VoteCommand) -> Self {
        Self { vote_command }
    }

    pub fn active_voting_committees<S: Into<String>>(mut self, host: S) -> Vec<CommitteeIdDef> {
        let content = self
            .vote_command
            .active_committees(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("JCLI returned malformed CommitteeIdDef")
    }

    pub fn active_vote_plans<S: Into<String>>(mut self, host: S) -> Vec<Value> {
        let content = self
            .vote_command
            .active_vote_plans(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("JCLI returned malformed VotePlan")
    }
}
