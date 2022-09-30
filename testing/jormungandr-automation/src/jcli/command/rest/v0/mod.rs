mod block;
mod message;
mod node;
mod utxo;
mod vote;

use crate::jcli::command::TransactionCommand;
pub use block::BlockCommand;
pub use message::MessageCommand;
pub use node::NodeCommand;
use std::process::Command;
pub use utxo::UtxOCommand;
pub use vote::VoteCommand;

pub struct V0Command {
    command: Command,
}

impl V0Command {
    pub fn new(command: Command) -> Self {
        Self { command }
    }

    pub fn node(mut self) -> NodeCommand {
        self.command.arg("node");
        NodeCommand::new(self.command)
    }

    pub fn utxo(mut self) -> UtxOCommand {
        self.command.arg("utxo");
        UtxOCommand::new(self.command)
    }

    pub fn message(mut self) -> MessageCommand {
        self.command.arg("message");
        MessageCommand::new(self.command)
    }

    pub fn block(mut self) -> BlockCommand {
        self.command.arg("block");
        BlockCommand::new(self.command)
    }

    pub fn vote(mut self) -> VoteCommand {
        self.command.arg("vote");
        VoteCommand::new(self.command)
    }

    pub fn transaction(mut self) -> TransactionCommand {
        self.command.arg("transaction");
        TransactionCommand::new(self.command)
    }

    pub fn leadership_log<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("leaders")
            .arg("logs")
            .arg("get")
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn tip<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("tip")
            .arg("get")
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn account_stats<S: Into<String>, P: Into<String>>(mut self, address: S, host: P) -> Self {
        self.command
            .arg("account")
            .arg("get")
            .arg(address.into())
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn shutdown<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("shutdown")
            .arg("post")
            .arg("-h")
            .arg(host.into());
        self
    }

    pub fn settings<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("settings")
            .arg("get")
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn stake_pools<S: Into<String>>(mut self, host: S) -> Self {
        self.command
            .arg("stake-pools")
            .arg("get")
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn stake_pool<S: Into<String>, P: Into<String>>(
        mut self,
        stake_pool_id: S,
        host: P,
    ) -> Self {
        self.command
            .arg("stake-pool")
            .arg("get")
            .arg(stake_pool_id.into())
            .arg("--host")
            .arg(host.into());
        self
    }

    pub fn build(self) -> Command {
        println!("{:?}", self.command);
        self.command
    }
}
