use crate::common::configuration::{
    genesis_model::{Fund, GenesisYaml},
    jormungandr_config::JormungandrConfig,
    node_config_model::{Logger, NodeConfig, Peer},
    secret_model::SecretModel,
};
use crate::common::file_utils;
use crate::common::jcli_wrapper;
pub struct ConfigurationBuilder {
    funds: Vec<Fund>,
    with_account: bool,
    trusted_peers: Option<Vec<Peer>>,
    block0_hash: Option<String>,
    block0_consensus: Option<String>,
    logger: Option<Logger>,
    bft_slots_ratio: Option<String>,
    consensus_genesis_praos_active_slot_coeff: Option<String>,
    kes_update_speed: u32,
    certs: Vec<String>,
    consensus_leader_ids: Vec<String>,
}

impl ConfigurationBuilder {
    pub fn new() -> Self {
        ConfigurationBuilder {
            funds: vec![],
            certs: vec![],
            consensus_leader_ids: vec![],
            with_account: false,
            trusted_peers: None,
            block0_hash: None,
            block0_consensus: Some("bft".to_string()),
            logger: None,
            bft_slots_ratio: Some("0.222".to_owned()),
            consensus_genesis_praos_active_slot_coeff: Some("0.1".to_owned()),
            kes_update_speed: 12 * 3600,
        }
    }

    pub fn with_kes_update_speed<'a>(&'a mut self, kes_update_speed: u32) -> &'a mut Self {
        self.kes_update_speed = kes_update_speed;
        self
    }

    pub fn with_consensus_leaders_ids<'a>(
        &'a mut self,
        consensus_leader_ids: Vec<String>,
    ) -> &'a mut Self {
        self.consensus_leader_ids = consensus_leader_ids;
        self
    }

    pub fn with_initial_certs<'a>(&'a mut self, certs: Vec<String>) -> &'a mut Self {
        self.certs = certs;
        self
    }

    pub fn with_allow_account_creation<'a>(&'a mut self, b: bool) -> &'a mut Self {
        self.with_account = b;
        self
    }

    pub fn with_block0_consensus<'a>(&'a mut self, consensus: &str) -> &'a mut Self {
        self.block0_consensus = Some(consensus.to_string());
        self
    }

    pub fn with_consensus_genesis_praos_active_slot_coeff<'a>(
        &'a mut self,
        active_slot_coeff: &str,
    ) -> &'a mut Self {
        self.consensus_genesis_praos_active_slot_coeff = Some(active_slot_coeff.to_string());
        self
    }

    pub fn with_bft_slots_ratio<'a>(&'a mut self, slots_ratio: String) -> &'a mut Self {
        self.bft_slots_ratio = Some(slots_ratio);
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

        let mut leaders_ids = vec![public_key];
        leaders_ids.append(&mut self.consensus_leader_ids.clone());
        genesis_model.blockchain_configuration.consensus_leader_ids = Some(leaders_ids.clone());
        genesis_model
            .blockchain_configuration
            .allow_account_creation = self.with_account;
        genesis_model.blockchain_configuration.block0_consensus = self.block0_consensus.clone();
        genesis_model.blockchain_configuration.bft_slots_ratio = self.bft_slots_ratio.clone();
        genesis_model.blockchain_configuration.kes_update_speed = self.kes_update_speed.clone();
        genesis_model
            .blockchain_configuration
            .consensus_genesis_praos_active_slot_coeff =
            self.consensus_genesis_praos_active_slot_coeff.clone();
        genesis_model.initial_certs = self.certs.clone();
        let path_to_output_block = super::build_genesis_block(&genesis_model);

        let mut config = JormungandrConfig::from(genesis_model, node_config);

        let secret_model = SecretModel::new_bft(&secret_key);
        let secret_model_path = SecretModel::serialize(&secret_model);

        config.secret_model = secret_model;
        config.secret_model_path = secret_model_path;
        config.genesis_block_path = path_to_output_block.clone();
        config.node_config_path = node_config_path;
        config.log_file_path = file_utils::get_path_in_temp("log_file.log");

        config.genesis_block_hash = match self.block0_hash {
            Some(ref value) => value.clone(),
            None => jcli_wrapper::assert_genesis_hash(&path_to_output_block),
        };
        config
    }
}
