use super::{FromGenesis, Role};

use jormungandr_testing_utils::testing::{JormungandrParams, TestConfig};
use serde::Serialize;
use std::iter::FromIterator;
use std::path::Path;
use std::process::Command;

pub struct CommandBuilder<'a> {
    bin: &'a Path,
    config: Option<&'a Path>,
    genesis_block: GenesisBlockOption<'a>,
    secrets: Vec<&'a Path>,
    log_file: Option<&'a Path>,
    rewards_history: bool,
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
            secrets: Vec::new(),
            log_file: None,
            rewards_history: false,
        }
    }

    pub fn config(mut self, path: &'a Path) -> Self {
        self.config = Some(path);
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

    pub fn leader_with_secrets<Iter>(mut self, secrets: Iter) -> Self
    where
        Iter: IntoIterator<Item = &'a Path>,
    {
        self.secrets = Vec::from_iter(secrets);
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
        let mut command = Command::new(self.bin);
        for secret_path in self.secrets {
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

        println!("Running start jormungandr command: {:?}", &command);
        command
    }
}

pub fn get_command<Conf: TestConfig + Serialize>(
    params: &JormungandrParams<Conf>,
    bin_path: impl AsRef<Path>,
    role: Role,
    from_genesis: FromGenesis,
) -> Command {
    let bin_path = bin_path.as_ref();
    let mut builder = CommandBuilder::new(bin_path)
        .config(params.node_config_path())
        .rewards_history(params.rewards_history());
    if params.node_config().log_file_path().is_none() {
        builder = builder.stderr_to_log_file(params.log_file_path());
    }
    let builder = match (role, from_genesis) {
        (Role::Passive, _) => builder.genesis_block_hash(params.genesis_block_hash()),
        (Role::Leader, FromGenesis::File) => builder
            .genesis_block_path(params.genesis_block_path())
            .leader_with_secrets(params.secret_model_paths()),
        (Role::Leader, FromGenesis::Hash) => builder
            .genesis_block_hash(params.genesis_block_hash())
            .leader_with_secrets(params.secret_model_paths()),
    };
    builder.command()
}
