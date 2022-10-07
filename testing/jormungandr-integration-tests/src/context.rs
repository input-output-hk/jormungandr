#![allow(dead_code)]

use assert_fs::TempDir;
use jormungandr_automation::jormungandr::{
    JormungandrBootstrapper, JormungandrProcess, LegacyNodeConfig, SecretModelFactory, Starter,
    StartupError, Version,
};
use jormungandr_lib::interfaces::{Block0Configuration, NodeConfig};
use std::path::PathBuf;
use thor::Wallet;

pub struct TestContext {
    pub block0_config: Block0Configuration,
    pub node_config: NodeConfig,
    pub secret_factory: SecretModelFactory,
    pub reward_history: bool,
}

impl TestContext {
    pub fn restart_node(
        &self,
        mut jormungandr: JormungandrProcess,
    ) -> Result<JormungandrProcess, StartupError> {
        let temp_dir = jormungandr.steal_temp_dir().unwrap().try_into().unwrap();
        self.start_node(temp_dir)
    }

    pub fn node_config(&self) -> NodeConfig {
        self.node_config.clone()
    }

    pub fn start_node(&self, temp_dir: TempDir) -> Result<JormungandrProcess, StartupError> {
        let mut bootstrapper = JormungandrBootstrapper::default()
            .with_block0_configuration(self.block0_config.clone())
            .with_node_config(self.node_config.clone())
            .with_secret(self.secret_factory.clone());
        if self.reward_history {
            bootstrapper = bootstrapper.with_rewards_history();
        }
        bootstrapper.start(temp_dir)
    }

    pub(crate) fn starter(&self, temp_dir: TempDir) -> Result<Starter, StartupError> {
        let mut bootstrapper = JormungandrBootstrapper::default()
            .with_block0_configuration(self.block0_config.clone())
            .with_node_config(self.node_config.clone())
            .with_secret(self.secret_factory.clone());
        if self.reward_history {
            bootstrapper = bootstrapper.with_rewards_history();
        }
        bootstrapper.into_starter(temp_dir).map_err(Into::into)
    }

    pub(crate) fn block0_config(&self) -> Block0Configuration {
        self.block0_config.clone()
    }
}

pub struct ActorsTestContext {
    pub(crate) test_context: TestContext,
    pub(crate) alice: Option<Wallet>,
    pub(crate) bob: Option<Wallet>,
}

impl ActorsTestContext {
    pub(crate) fn start_node(&self, temp_dir: TempDir) -> Result<JormungandrProcess, StartupError> {
        self.test_context.start_node(temp_dir)
    }

    pub(crate) fn block0_config(&self) -> Block0Configuration {
        self.test_context.block0_config()
    }

    pub fn alice(&self) -> Wallet {
        self.alice.clone().expect("alice not defined")
    }

    pub(crate) fn bob(&self) -> Wallet {
        self.bob.clone().expect("bob not defined")
    }
}

pub struct LegacyTestContext {
    pub test_context: TestContext,
    pub legacy_node_config: LegacyNodeConfig,
    pub jormungandr_app: Option<PathBuf>,
    pub version: Version,
}

impl LegacyTestContext {
    pub(crate) fn start_node(&self, temp_dir: TempDir) -> Result<JormungandrProcess, StartupError> {
        let mut bootstrapper = JormungandrBootstrapper::default()
            .with_block0_configuration(self.block0_config())
            .with_legacy_node_config(self.legacy_node_config.clone())
            .with_secret(self.test_context.secret_factory.clone());
        if self.test_context.reward_history {
            bootstrapper = bootstrapper.with_rewards_history();
        }
        bootstrapper.start(temp_dir)
    }

    pub(crate) fn starter(&self, temp_dir: TempDir) -> Result<Starter, StartupError> {
        let mut bootstrapper = JormungandrBootstrapper::default()
            .with_block0_configuration(self.block0_config())
            .with_legacy_node_config(self.legacy_node_config.clone())
            .with_secret(self.test_context.secret_factory.clone());
        if self.test_context.reward_history {
            bootstrapper = bootstrapper.with_rewards_history();
        }
        let mut starter = bootstrapper.into_starter(temp_dir)?;
        if let Some(jormungandr) = &self.jormungandr_app {
            starter = starter.jormungandr_app(jormungandr.to_path_buf());
        }
        Ok(starter.verbose(true))
    }

    pub(crate) fn block0_config(&self) -> Block0Configuration {
        self.test_context.block0_config()
    }
}
