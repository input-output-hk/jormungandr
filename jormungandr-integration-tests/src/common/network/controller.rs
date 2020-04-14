use crate::common::configuration::jormungandr_config::JormungandrConfig;
use crate::common::file_utils;
use crate::common::jormungandr::{
    process::JormungandrProcess,
    starter::{Starter, StartupError},
};
use chain_impl_mockchain::header::HeaderId;
use jormungandr_lib::testing::network_builder::{
    LeadershipMode, PersistenceMode, Settings, Wallet,
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

    pub fn spawn_and_wait(&mut self, alias: &str) -> JormungandrProcess {
        self.spawn_node(alias, PersistenceMode::InMemory, LeadershipMode::Leader)
            .expect(&format!("cannot start {}", alias))
    }

    pub fn spawn_node(
        &mut self,
        alias: &str,
        persistence_mode: PersistenceMode,
        leadership_mode: LeadershipMode,
    ) -> Result<JormungandrProcess, ControllerError> {
        let node_setting = if let Some(node_setting) = self.settings.nodes.get(alias.clone()) {
            node_setting
        } else {
            return Err(ControllerError::NodeNotFound(alias.to_string()));
        };

        let mut settings = node_setting.clone();
        let dir = file_utils::get_path_in_temp("network").join(&settings.alias);

        if let PersistenceMode::Persistent = persistence_mode {
            let path_to_storage = dir.join("storage");
            settings.config.storage = Some(path_to_storage);
        }

        std::fs::DirBuilder::new().recursive(true).create(&dir)?;

        let config_file = dir.join("node_config.xml");
        let config_secret = dir.join("node_secret.xml");

        serde_yaml::to_writer(std::fs::File::create(&config_file)?, settings.config())?;

        serde_yaml::to_writer(std::fs::File::create(&config_secret)?, settings.secrets())?;

        let config = JormungandrConfig {
            genesis_block_path: self.block0_file.as_path().into(),
            genesis_block_hash: self.block0_hash.to_string(),
            node_config_path: config_file,
            secret_model_paths: vec![config_secret],
            block0_configuration: self.settings.block0.clone(),
            node_config: node_setting.config.clone(),
            secret_models: vec![node_setting.secrets().clone()],
            log_file_path: file_utils::get_path_in_temp("log_file.log"),
            rewards_history: false,
        };

        Starter::new()
            .config(config)
            .from_genesis(leadership_mode.clone().into())
            .role(leadership_mode.into())
            .start()
            .map_err(|e| ControllerError::SpawnError(e))
    }
}
