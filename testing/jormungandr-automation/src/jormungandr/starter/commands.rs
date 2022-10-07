#![allow(dead_code)]
use crate::jormungandr::{
    starter::{JormungandrParams, NodeBlock0},
    FaketimeConfig, LeadershipMode,
};
use jormungandr_lib::crypto::hash::Hash;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub struct CommandBuilder {
    bin: PathBuf,
    config: Option<PathBuf>,
    genesis_block: Option<NodeBlock0>,
    secret: Option<PathBuf>,
    log_file: Option<PathBuf>,
    rewards_history: bool,
    faketime: Option<FaketimeConfig>,
}

impl CommandBuilder {
    pub fn new(bin: &Path) -> Self {
        CommandBuilder {
            bin: bin.to_path_buf(),
            config: None,
            genesis_block: None,
            secret: None,
            log_file: None,
            rewards_history: false,
            faketime: None,
        }
    }

    pub fn config(mut self, path: &Path) -> Self {
        self.config = Some(path.to_path_buf());
        self
    }

    pub fn faketime(mut self, faketime: FaketimeConfig) -> Self {
        self.faketime = Some(faketime);
        self
    }

    pub fn genesis_block_hash(mut self, hash: Hash) -> Self {
        self.genesis_block = Some(NodeBlock0::Hash(hash));
        self
    }

    pub fn genesis_block_path(mut self, path: &Path) -> Self {
        self.genesis_block = Some(NodeBlock0::File(path.to_path_buf()));
        self
    }

    pub fn leader_with_secret(mut self, secret: &Path) -> Self {
        self.secret = Some(secret.to_path_buf());
        self
    }

    pub fn stderr_to_log_file(mut self, path: &Path) -> Self {
        self.log_file = Some(path.to_path_buf());
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

        if let Some(node_block0) = &self.genesis_block {
            match node_block0 {
                NodeBlock0::Hash(hash) => {
                    command.arg("--genesis-block-hash").arg(hash.to_string());
                }
                NodeBlock0::File(path) => {
                    command.arg("--genesis-block").arg(path);
                }
            }
        } else {
            panic!("one of the genesis block options needs to be specified")
        }

        command.stderr(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());

        command
    }
}

pub fn get_command(
    params: &JormungandrParams,
    bin_path: impl AsRef<Path>,
    leadership_mode: LeadershipMode,
) -> Command {
    let bin_path = bin_path.as_ref();
    let node_config_path = params.node_config_path();
    let secret_path = params.secret_path();
    let builder = CommandBuilder::new(bin_path).config(&node_config_path);

    let builder = match (leadership_mode, params.genesis()) {
        (LeadershipMode::Passive, NodeBlock0::Hash(hash)) => builder.genesis_block_hash(*hash),
        (LeadershipMode::Leader, NodeBlock0::File(block_path)) => {
            builder.genesis_block_path(block_path).leader_with_secret(
                secret_path
                    .as_ref()
                    .expect("no secrets defined for leader node"),
            )
        }
        (LeadershipMode::Leader, NodeBlock0::Hash(block_hash)) => {
            builder.genesis_block_hash(*block_hash).leader_with_secret(
                secret_path
                    .as_ref()
                    .expect("no secrets defined for leader node"),
            )
        }
        (LeadershipMode::Passive, NodeBlock0::File(block_path)) => {
            builder.genesis_block_path(block_path)
        }
    };
    builder.rewards_history(params.rewards_history()).command()
}
