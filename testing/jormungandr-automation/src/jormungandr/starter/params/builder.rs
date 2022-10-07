use crate::jormungandr::{
    configuration::{NodeConfigBuilder, NodeConfigManager},
    starter::{params::Error, JormungandrParams, NodeBlock0},
    ConfigurableNodeConfig, EitherHashOrBlock0, JormungandrProcess, LeadershipMode,
    LegacyNodeConfig, LegacyNodeConfigManager, SecretModelFactory, Starter, StartupError,
};
use assert_fs::{fixture::PathChild, TempDir};
use chain_core::{packer::Codec, property::Serialize};
use chain_crypto::Ed25519;
use jormungandr_lib::{
    crypto::{hash::Hash, key::KeyPair},
    interfaces::{Block0Configuration, NodeConfig},
};
use std::{fs::File, path::PathBuf};

#[derive(Debug)]
pub struct JormungandrBootstrapper {
    pub node_config: Box<dyn ConfigurableNodeConfig>,
    pub genesis: Option<EitherHashOrBlock0>,
    pub secret: SecretModelFactory,
    pub leadership_mode: LeadershipMode,
    pub jormungandr_app: Option<PathBuf>,
    pub rewards_history: bool,
    pub verbose: bool,
}

impl Default for JormungandrBootstrapper {
    fn default() -> Self {
        Self {
            node_config: Box::new(NodeConfigManager {
                node_config: NodeConfigBuilder::default().build(),
                file: None,
            }),
            genesis: Default::default(),
            secret: Default::default(),
            leadership_mode: LeadershipMode::Leader,
            jormungandr_app: None,
            verbose: true,
            rewards_history: false,
        }
    }
}

impl JormungandrBootstrapper {
    pub fn passive(mut self) -> Self {
        self.leadership_mode = LeadershipMode::Passive;
        self
    }

    pub fn with_rewards_history(mut self) -> Self {
        self.rewards_history = true;
        self
    }

    pub fn with_leader_key(self, leader: &KeyPair<Ed25519>) -> Self {
        self.with_secret(SecretModelFactory::bft(leader.signing_key()))
    }

    pub fn with_secret(mut self, secret: SecretModelFactory) -> Self {
        self.secret = secret;
        self
    }

    pub fn into_starter(self, temp_dir: TempDir) -> Result<Starter, Error> {
        Ok(Starter::default()
            .verbose(self.verbose)
            .config(self.build(&temp_dir)?)
            .temp_dir(temp_dir))
    }

    pub fn with_node_config(mut self, node_config: NodeConfig) -> Self {
        self.node_config = Box::new(NodeConfigManager {
            node_config,
            file: None,
        });
        self
    }

    pub fn with_legacy_node_config(mut self, node_config: LegacyNodeConfig) -> Self {
        self.node_config = Box::new(LegacyNodeConfigManager {
            node_config,
            file: None,
        });
        self
    }

    pub fn with_block0_hash(mut self, block0_hash: Hash) -> Self {
        self.genesis = Some(EitherHashOrBlock0::Hash(block0_hash));
        self
    }

    pub fn with_block0_configuration(mut self, block0_config: Block0Configuration) -> Self {
        self.genesis = Some(EitherHashOrBlock0::Block0(block0_config));
        self
    }

    pub fn verbose(mut self) -> Self {
        self.verbose = true;
        self
    }

    pub fn build(mut self, temp_dir: &impl PathChild) -> Result<JormungandrParams, Error> {
        let genesis_source = &self.genesis.ok_or(Error::Block0SourceNotDefined)?;

        let genesis = match genesis_source {
            EitherHashOrBlock0::Hash(hash) => NodeBlock0::Hash(*hash),
            EitherHashOrBlock0::Block0(ref block0) => {
                let block0_bin = temp_dir.child("block0.bin");
                let file = File::create(block0_bin.path())?;
                block0.to_block().serialize(&mut Codec::new(file))?;

                NodeBlock0::File(block0_bin.to_path_buf())
            }
        };

        let secret = temp_dir.child("secret");
        let secret_path = self.secret.write_to_file_if_defined(&secret);

        crate::cond_println!(self.verbose, "Node settings configuration:");
        crate::cond_println!(self.verbose, "{:#?}", self.node_config);

        crate::cond_println!(self.verbose, "Blockchain configuration:");
        crate::cond_println!(self.verbose, "{:#?}", genesis_source);

        crate::cond_println!(self.verbose, "Secret:");
        crate::cond_println!(self.verbose, "{:#?}", self.secret);

        let config_file = temp_dir.child("node_config.yaml");
        self.node_config
            .as_mut()
            .set_node_config_path(config_file.to_path_buf());
        self.node_config.as_ref().write_node_config();

        Ok(JormungandrParams {
            node_config: self.node_config,
            genesis,
            secret_path,
            leadership: self.leadership_mode,
            rewards_history: self.rewards_history,
        })
    }

    pub fn start(self, temp_dir: TempDir) -> Result<JormungandrProcess, StartupError> {
        self.into_starter(temp_dir)?.start()
    }
}

fn create_new_leader_key() -> KeyPair<Ed25519> {
    KeyPair::generate(&mut rand::thread_rng())
}
