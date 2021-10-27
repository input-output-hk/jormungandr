use crate::testing::BlockDateGenerator;
use crate::testing::DummySyncNode;
use crate::testing::FragmentSenderSetup;
use crate::testing::{
    jormungandr::starter::{Starter, StartupError},
    jormungandr::JormungandrProcess,
    node::LogLevel,
};
use crate::{
    testing::{
        network::{NodeSetting, PersistenceMode, Settings, SpawnParams},
        FragmentSender, JormungandrParams,
    },
    wallet::Wallet,
};
use assert_fs::fixture::FixtureError;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use chain_impl_mockchain::header::BlockDate;
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::interfaces::{Log, LogEntry, LogOutput, NodeConfig};
use std::path::PathBuf;
use thiserror::Error;

const NODE_CONFIG_FILE: &str = "node_config.yaml";
const NODE_SECRETS_FILE: &str = "node_secret.yaml";
const NODE_TOPOLOGY_KEY_FILE: &str = "node_topology_key";

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("node not found {0}")]
    NodeNotFound(String),
    #[error("wallet not found {0}")]
    WalletNotFound(String),
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("fixture filesystem error")]
    FsFixture(#[from] FixtureError),
    #[error("serialization error")]
    Serialization(#[from] serde_yaml::Error),
    #[error("node startup error")]
    Spawn(#[from] StartupError),
}

pub struct Controller {
    settings: Settings,
    working_directory: TempDir,
    block0_file: PathBuf,
    block0_hash: HeaderId,
}

impl Controller {
    pub fn new(settings: Settings, working_directory: TempDir) -> Result<Self, ControllerError> {
        use chain_core::property::Serialize as _;

        let block0 = settings.block0.to_block();
        let block0_hash = block0.header().hash();

        let block0_file = working_directory.child("block0.bin").path().into();
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;

        Ok(Controller {
            settings,
            working_directory,
            block0_file,
            block0_hash,
        })
    }

    pub fn wallet(&mut self, wallet: &str) -> Result<Wallet, ControllerError> {
        if let Some(wallet) = self.settings.wallets.remove(wallet) {
            Ok(wallet.into())
        } else {
            Err(ControllerError::WalletNotFound(wallet.to_owned()))
        }
    }

    pub fn node_config(&self, alias: &str) -> Result<NodeConfig, ControllerError> {
        Ok(self.node_settings(alias)?.config.clone())
    }

    pub fn node_settings(&self, alias: &str) -> Result<&NodeSetting, ControllerError> {
        self.settings
            .nodes
            .get(alias)
            .ok_or_else(|| ControllerError::NodeNotFound(alias.to_string()))
    }

    pub fn fragment_sender<'a>(&self) -> FragmentSender<'a, DummySyncNode> {
        FragmentSender::new(
            self.settings.block0.to_block().header().hash().into(),
            self.settings.block0.blockchain_configuration.linear_fees,
            self.default_block_date_generator(),
            FragmentSenderSetup::resend_3_times(),
        )
    }

    pub fn default_block_date_generator(&self) -> BlockDateGenerator {
        BlockDateGenerator::rolling_from_blockchain_config(
            &self.settings.block0.blockchain_configuration,
            BlockDate {
                epoch: 1,
                slot_id: 0,
            },
            false,
        )
    }

    pub fn spawn_node_async(&mut self, alias: &str) -> Result<JormungandrProcess, ControllerError> {
        let mut starter = self.make_starter_for(
            SpawnParams::new(alias).persistence_mode(PersistenceMode::InMemory),
        )?;
        let process = starter.start_async()?;
        Ok(process)
    }

    pub fn expect_spawn_failed(
        &mut self,
        spawn_params: SpawnParams,
        expected_msg: &str,
    ) -> Result<(), ControllerError> {
        let mut starter = self.make_starter_for(spawn_params)?;
        starter.start_with_fail_in_logs(expected_msg)?;
        Ok(())
    }

    pub fn spawn(
        &mut self,
        spawn_params: SpawnParams,
    ) -> Result<JormungandrProcess, ControllerError> {
        Ok(self.make_starter_for(spawn_params)?.start()?)
    }

    fn make_starter_for(
        &mut self,
        mut spawn_params: SpawnParams,
    ) -> Result<Starter, ControllerError> {
        let node_key_file = self
            .working_directory
            .child(spawn_params.get_alias())
            .child(NODE_TOPOLOGY_KEY_FILE)
            .path()
            .into();

        spawn_params = spawn_params.node_key_file(node_key_file);

        let node_setting = self.node_settings(spawn_params.get_alias())?;
        let dir = self.working_directory.child(spawn_params.get_alias());
        let mut config = node_setting.config.clone();
        spawn_params.override_settings(&mut config);

        for peer in config.p2p.trusted_peers.iter_mut() {
            peer.id = None;
        }

        let log_file_path = dir.child("node.log").path().to_path_buf();
        config.log = Some(Log(LogEntry {
            format: "json".into(),
            level: spawn_params
                .get_log_level()
                .unwrap_or(&LogLevel::DEBUG)
                .to_string(),
            output: LogOutput::Stdout,
        }));

        if let PersistenceMode::Persistent = spawn_params.get_persistence_mode() {
            let path_to_storage = dir.child("storage").path().into();
            config.storage = Some(path_to_storage);
        }
        dir.create_dir_all()?;

        let config_file = dir.child(NODE_CONFIG_FILE);
        let yaml = serde_yaml::to_string(&config)?;
        config_file.write_str(&yaml)?;

        let secret_file = dir.child(NODE_SECRETS_FILE);
        let yaml = serde_yaml::to_string(&node_setting.secret)?;
        secret_file.write_str(&yaml)?;

        let topology_file = dir.child(NODE_TOPOLOGY_KEY_FILE);
        topology_file.write_str(&node_setting.topology_secret.to_bech32_str())?;

        let params = JormungandrParams::new(
            config,
            config_file.path(),
            &self.block0_file,
            self.block0_hash.to_string(),
            secret_file.path(),
            self.settings.block0.clone(),
            false,
            log_file_path,
        );

        let mut starter = Starter::new();
        starter
            .config(params)
            .alias(spawn_params.get_alias().clone())
            .from_genesis(spawn_params.get_leadership_mode().into())
            .leadership_mode(spawn_params.get_leadership_mode());
        Ok(starter)
    }
}
