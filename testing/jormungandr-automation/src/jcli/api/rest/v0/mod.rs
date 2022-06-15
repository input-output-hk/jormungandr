mod block;
mod message;
mod node;
mod utxo;
mod vote;

use crate::jcli::command::rest::V0Command;
use assert_cmd::assert::OutputAssertExt;
use block::Block;
use jormungandr_lib::interfaces::{AccountState, LeadershipLog, SettingsDto, StakePoolStats};
use jortestkit::prelude::ProcessOutput;
use message::Message;
use node::Node;
use utxo::UtxO;
use vote::Vote;
pub struct RestV0 {
    v0_command: V0Command,
}

impl RestV0 {
    pub fn new(v0_command: V0Command) -> Self {
        Self { v0_command }
    }

    pub fn utxo(self) -> UtxO {
        UtxO::new(self.v0_command.utxo())
    }

    pub fn node(self) -> Node {
        Node::new(self.v0_command.node())
    }

    pub fn message(self) -> Message {
        Message::new(self.v0_command.message())
    }

    pub fn block(self) -> Block {
        Block::new(self.v0_command.block())
    }

    pub fn vote(self) -> Vote {
        Vote::new(self.v0_command.vote())
    }

    pub fn settings<S: Into<String>>(self, host: S) -> SettingsDto {
        let content = self
            .v0_command
            .settings(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("Failed to parse settings")
    }

    pub fn stake_pools<S: Into<String>>(self, host: S) -> Vec<String> {
        let content = self
            .v0_command
            .stake_pools(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("Failed to parse stake poools collection")
    }

    pub fn stake_pool<S: Into<String>, P: Into<String>>(
        self,
        stake_pool_id: S,
        host: P,
    ) -> StakePoolStats {
        let content = self
            .v0_command
            .stake_pool(stake_pool_id, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).expect("Failed to parse stak pool stats")
    }

    pub fn leadership_log<S: Into<String>>(self, host: S) -> Vec<LeadershipLog> {
        let content = self
            .v0_command
            .leadership_log(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        println!("Output: {:?}", content);
        serde_yaml::from_str(&content).unwrap()
    }

    pub fn tip<S: Into<String>>(self, host: S) -> String {
        self.v0_command
            .tip(host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_single_line()
    }

    pub fn tip_expect_fail<S: Into<String>>(self, host: S, expected_msg: &str) {
        self.v0_command
            .tip(host)
            .build()
            .assert()
            .failure()
            .stderr(predicates::str::contains(expected_msg));
    }

    pub fn account_stats<S: Into<String>, P: Into<String>>(
        self,
        address: S,
        host: P,
    ) -> AccountState {
        let content = self
            .v0_command
            .account_stats(address, host)
            .build()
            .assert()
            .success()
            .get_output()
            .as_lossy_string();
        serde_yaml::from_str(&content).unwrap()
    }

    pub fn shutdown<S: Into<String>>(self, host: S) {
        self.v0_command.shutdown(host).build().assert().success();
    }
}
