use crate::common::{
    configuration::jormungandr_config::JormungandrConfig,
    file_utils,
    jormungandr::starter::{Starter, StartupError},
    network::Node,
};
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::interfaces::NodeConfig;
use jormungandr_lib::testing::network_builder::NodeSetting;
use jormungandr_lib::testing::network_builder::{
    LeadershipMode, PersistenceMode, Settings, SpawnParams, Wallet,
};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ControllerError {
    #[error("node not found {0}")]
    NodeNotFound(String),
    #[error("wallet not found {0}")]
    WalletNotFound(String),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("serialization error")]
    SerializationError(#[from] serde_yaml::Error),
    #[error("serialization error")]
    SpawnError(#[from] StartupError),
}

pub struct Controller {
    settings: Settings,
    working_directory: PathBuf,
    block0_file: PathBuf,
    block0_hash: HeaderId,
}

impl Controller {
    pub fn new(
        title: &str,
        settings: Settings,
        working_directory: PathBuf,
    ) -> Result<Self, ControllerError> {
        let working_directory = working_directory.join(title);
        std::fs::DirBuilder::new()
            .recursive(true)
            .create(&working_directory)?;

        use chain_core::property::Serialize as _;

        let block0 = settings.block0.to_block();
        let block0_hash = block0.header.hash();

        let block0_file = working_directory.join("block0.bin");
        let file = std::fs::File::create(&block0_file)?;
        block0.serialize(file)?;

        Ok(Controller {
            settings: settings,
            block0_file,
            block0_hash,
            working_directory,
        })
    }

    pub fn wallet(&mut self, wallet: &str) -> Result<Wallet, ControllerError> {
        if let Some(wallet) = self.settings.wallets.remove(wallet) {
            Ok(wallet)
        } else {
            Err(ControllerError::WalletNotFound(wallet.to_owned()).into())
        }
    }

    pub fn node_config(&self, alias: &str) -> Result<NodeConfig, ControllerError> {
        Ok(self.node_settings(alias)?.config.clone())
    }

    fn node_settings(&self, alias: &str) -> Result<&NodeSetting, ControllerError> {
        if let Some(node_setting) = self.settings.nodes.get(alias) {
            return Ok(node_setting);
        } else {
            return Err(ControllerError::NodeNotFound(alias.to_string()));
        }
    }

    pub fn spawn_and_wait(&mut self, alias: &str) -> Node {
        self.spawn_node(alias, PersistenceMode::InMemory, LeadershipMode::Leader)
            .expect(&format!("cannot start {}", alias))
    }

    
    pub fn spawn_as_passive_and_wait(&mut self, alias: &str) -> Node {
        self.spawn_node(alias, PersistenceMode::InMemory, LeadershipMode::Passive)
            .expect(&format!("cannot start {}", alias))
    }

    pub fn expect_spawn_failed(
        &mut self,
        spawn_params: &mut SpawnParams,
        expected_msg: &str,
    ) -> Result<(), ControllerError> {
        let config = self.make_config_for(spawn_params).unwrap();
        Starter::new()
            .config(config)
            .from_genesis(spawn_params.get_leadership_mode().clone().into())
            .role(spawn_params.get_leadership_mode().into())
            .start_with_fail_in_logs(expected_msg)
            .map_err(|e| ControllerError::SpawnError(e))
    }

    pub fn spawn_custom(
        &mut self,
        spawn_params: &mut SpawnParams,
    ) -> Result<Node, ControllerError> {
        let config = self.make_config_for(spawn_params).unwrap();
        Starter::new()
            .config(config)
            .from_genesis(spawn_params.get_leadership_mode().clone().into())
            .role(spawn_params.get_leadership_mode().into())
            .start()
            .map_err(|e| ControllerError::SpawnError(e))
            .map(|jormungandr| Node::new(jormungandr, &spawn_params.alias))
    }

    pub fn make_config_for(
        &mut self,
        spawn_params: &mut SpawnParams,
    ) -> Result<JormungandrConfig, ControllerError> {
        let mut node_setting = self.node_settings(&spawn_params.alias)?.clone();
        spawn_params.override_settings(&mut node_setting.config);

        let dir = file_utils::get_path_in_temp("network").join(&node_setting.alias);

        if let PersistenceMode::Persistent = spawn_params.get_persistence_mode() {
            let path_to_storage = dir.join("storage");
            node_setting.config.storage = Some(path_to_storage);
        }

        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let config_file = dir.join("node_config.xml");
        let config_secret = dir.join("node_secret.xml");

        serde_yaml::to_writer(std::fs::File::create(&config_file)?, node_setting.config())?;

        serde_yaml::to_writer(
            std::fs::File::create(&config_secret)?,
            node_setting.secrets(),
        )?;

        Ok(JormungandrConfig {
            genesis_block_path: self.block0_file.as_path().into(),
            genesis_block_hash: self.block0_hash.to_string(),
            node_config_path: config_file,
            secret_model_paths: vec![config_secret],
            block0_configuration: self.settings.block0.clone(),
            node_config: node_setting.config.clone(),
            secret_models: vec![node_setting.secrets().clone()],
            log_file_path: file_utils::get_path_in_temp("log_file.log"),
            rewards_history: false,
        })
    }

    pub fn spawn_node(
        &mut self,
        alias: &str,
        persistence_mode: PersistenceMode,
        leadership_mode: LeadershipMode,
    ) -> Result<Node, ControllerError> {
        self.spawn_custom(
            SpawnParams::new(alias)
                .leadership_mode(leadership_mode)
                .persistence_mode(persistence_mode),
        )
    }
}
