#![cfg(feature = "integration-test")]

use common::jcli_wrapper;

use common::configuration::{
    genesis_model::{Fund, GenesisYaml},
    jormungandr_config::JormungandrConfig,
    node_config_model::{Logger, NodeConfig, Peer},
    secret_model::SecretModel,
};

pub struct ConfigurationBuilder {
    funds: Vec<Fund>,
    with_account: bool,
    trusted_peers: Option<Vec<Peer>>,
    block0_hash: Option<String>,
    logger: Option<Logger>,
}

impl ConfigurationBuilder {
    pub fn new() -> Self {
        ConfigurationBuilder {
            funds: vec![],
            with_account: false,
            trusted_peers: None,
            block0_hash: None,
            logger: None,
        }
    }

    pub fn with_allow_account_creation<'a>(&'a mut self, b: bool) -> &'a mut Self {
        self.with_account = b;
        self
    }

    pub fn with_funds<'a>(&'a mut self, funds: Vec<Fund>) -> &'a mut Self {
        self.funds = funds.clone();
        self
    }

    pub fn with_logger<'a>(&'a mut self, logger: Logger) -> &'a mut Self {
        self.logger = Some(logger.clone());
        self
    }

    pub fn with_trusted_peers<'a>(&'a mut self, trusted_peers: Vec<Peer>) -> &'a mut Self {
        self.trusted_peers = Some(trusted_peers.clone());
        self
    }

    pub fn with_block_hash<'a>(&'a mut self, block0_hash: String) -> &'a mut Self {
        self.block0_hash = Some(block0_hash.clone());
        self
    }

    pub fn build(&self) -> JormungandrConfig {
        let mut node_config = NodeConfig::new();
        node_config.peer_2_peer.trusted_peers = self.trusted_peers.clone();
        node_config.logger = self.logger.clone();
        let node_config_path = NodeConfig::serialize(&node_config);

        let secret_key = jcli_wrapper::assert_key_generate_default();
        let public_key = jcli_wrapper::assert_key_to_public_default(&secret_key);

        let mut genesis_model = GenesisYaml::new_with_funds(self.funds.clone());
        genesis_model.blockchain_configuration.consensus_leader_ids = Some(vec![public_key]);
        genesis_model
            .blockchain_configuration
            .allow_account_creation = self.with_account;
        let path_to_output_block = super::build_genesis_block(&genesis_model);

        let mut config = JormungandrConfig::from(genesis_model, node_config);

        let secret_model = SecretModel::new(&secret_key);
        let secret_model_path = SecretModel::serialize(&secret_model);

        config.secret_model = secret_model;
        config.secret_model_path = secret_model_path;
        config.genesis_block_path = path_to_output_block.clone();
        config.node_config_path = node_config_path;

        config.genesis_block_hash = match self.block0_hash {
            Some(ref value) => value.clone(),
            None => jcli_wrapper::assert_genesis_hash(&path_to_output_block),
        };
        config
    }
}
