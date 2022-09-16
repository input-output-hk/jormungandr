#![allow(dead_code)]
use crate::jormungandr::{
    starter::FromGenesis, FaketimeConfig, JormungandrParams, LeadershipMode, TestConfig,
};
use serde::Serialize;
use std::{path::Path, process::Command};

pub struct CommandBuilder<'a> {
    bin: &'a Path,
    config: Option<&'a Path>,
    genesis_block: GenesisBlockOption<'a>,
    secret: Option<&'a Path>,
    log_file: Option<&'a Path>,
    rewards_history: bool,
    faketime: Option<FaketimeConfig>,
}

enum GenesisBlockOption<'a> {
    None,
    Hash(&'a str),
    Path(&'a Path),
}

impl<'a> CommandBuilder<'a> {
    pub fn new(bin: &'a Path) -> Self {
        CommandBuilder {
            bin,
            config: None,
            genesis_block: GenesisBlockOption::None,
            secret: None,
            log_file: None,
            rewards_history: false,
            faketime: None,
        }
    }

    pub fn config(mut self, path: &'a Path) -> Self {
        self.config = Some(path);
        self
    }

    pub fn faketime(mut self, faketime: FaketimeConfig) -> Self {
        self.faketime = Some(faketime);
        self
    }

    pub fn genesis_block_hash(mut self, hash: &'a str) -> Self {
        self.genesis_block = GenesisBlockOption::Hash(hash);
        self
    }

    pub fn genesis_block_path(mut self, path: &'a Path) -> Self {
        self.genesis_block = GenesisBlockOption::Path(path);
        self
    }

    pub fn leader_with_secret(mut self, secret: &'a Path) -> Self {
        self.secret = Some(secret);
        self
    }

    pub fn stderr_to_log_file(mut self, path: &'a Path) -> Self {
        self.log_file = Some(path);
        self
    }

    pub fn rewards_history(mut self, report: bool) -> Self {
        self.rewards_history = report;
        self
    }

    pub fn command(self) -> Command {
        let mut command = if let Some(faketime) = &self.faketime {
            let mut cmd = Command::new("faketime");
            cmd.args(["-f", &format!("{:+}s", faketime.offset)]);
            cmd.arg(self.bin);
            cmd
        } else {
            Command::new(self.bin)
        };

        if let Some(secret_path) = self.secret {
            command.arg("--secret").arg(secret_path);
        }

        if self.rewards_history {
            command.arg("--rewards-report-all");
        }

        let config_path = self
            .config
            .expect("configuration file path needs to be set");
        command.arg("--config").arg(config_path);

        match self.genesis_block {
            GenesisBlockOption::Hash(hash) => {
                command.arg("--genesis-block-hash").arg(hash);
            }
            GenesisBlockOption::Path(path) => {
                command.arg("--genesis-block").arg(path);
            }
            GenesisBlockOption::None => {
                panic!("one of the genesis block options needs to be specified")
            }
        }

        command.stderr(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());

        command
    }
}

pub fn get_command<Conf: TestConfig + Serialize>(
    params: &JormungandrParams<Conf>,
    bin_path: impl AsRef<Path>,
    leadership_mode: LeadershipMode,
    from_genesis: FromGenesis,
) -> Command {
    let bin_path = bin_path.as_ref();
    let builder = CommandBuilder::new(bin_path)
        .config(params.node_config_path())
        .rewards_history(params.rewards_history());

    let builder = match (leadership_mode, from_genesis) {
        (LeadershipMode::Passive, _) => builder.genesis_block_hash(params.genesis_block_hash()),
        (LeadershipMode::Leader, FromGenesis::File) => builder
            .genesis_block_path(params.genesis_block_path())
            .leader_with_secret(params.secret_model_path()),
        (LeadershipMode::Leader, FromGenesis::Hash) => builder
            .genesis_block_hash(params.genesis_block_hash())
            .leader_with_secret(params.secret_model_path()),
    };
    builder.command()
}
