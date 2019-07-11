use crate::common::configuration::{
    genesis_model::{Fund, GenesisYaml, Initial, LinearFees},
    jormungandr_config::JormungandrConfig,
    node_config_model::{Log, NodeConfig, Peer},
    secret_model::SecretModel,
};
use crate::common::file_utils;
use crate::common::jcli_wrapper;
pub struct ConfigurationBuilder {
    funds: Vec<Fund>,
    trusted_peers: Option<Vec<Peer>>,
    block0_hash: Option<String>,
    block0_consensus: Option<String>,
    log: Option<Log>,
    bft_slots_ratio: Option<String>,
    consensus_genesis_praos_active_slot_coeff: Option<String>,
    slots_per_epoch: Option<u32>,
    slot_duration: Option<u32>,
    epoch_stability_depth: Option<u32>,
    kes_update_speed: u32,
    linear_fees: LinearFees,
    certs: Vec<String>,
    consensus_leader_ids: Vec<String>,
}

impl ConfigurationBuilder {
    pub fn new() -> Self {
        ConfigurationBuilder {
            funds: vec![],
            certs: vec![],
            consensus_leader_ids: vec![],
            trusted_peers: None,
            block0_hash: None,
            block0_consensus: Some("bft".to_string()),
            slots_per_epoch: None,
            slot_duration: None,
            epoch_stability_depth: None,
            log: None,
            linear_fees: LinearFees {
                constant: 0,
                coefficient: 0,
                certificate: 0,
            },
            bft_slots_ratio: Some("0.222".to_owned()),
            consensus_genesis_praos_active_slot_coeff: Some("0.1".to_owned()),
            kes_update_speed: 12 * 3600,
        }
    }

    pub fn with_slots_per_epoch<'a>(&'a mut self, slots_per_epoch: u32) -> &'a mut Self {
        self.slots_per_epoch = Some(slots_per_epoch);
        self
    }

    pub fn with_slot_duration<'a>(&'a mut self, slot_duration: u32) -> &'a mut Self {
        self.slot_duration = Some(slot_duration);
        self
    }

    pub fn with_epoch_stability_depth<'a>(
        &'a mut self,
        epoch_stability_depth: u32,
    ) -> &'a mut Self {
        self.epoch_stability_depth = Some(epoch_stability_depth);
        self
    }

    pub fn with_kes_update_speed<'a>(&'a mut self, kes_update_speed: u32) -> &'a mut Self {
        self.kes_update_speed = kes_update_speed;
        self
    }

    pub fn with_linear_fees<'a>(&'a mut self, linear_fees: LinearFees) -> &'a mut Self {
        self.linear_fees = linear_fees;
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

    pub fn with_log<'a>(&'a mut self, log: Log) -> &'a mut Self {
        self.log = Some(log.clone());
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
        node_config.log = self.log.clone();
        let node_config_path = NodeConfig::serialize(&node_config);

        let secret_key = jcli_wrapper::assert_key_generate("ed25519");
        let public_key = jcli_wrapper::assert_key_to_public_default(&secret_key);

        let mut genesis_model = GenesisYaml::new_with_funds(&self.funds);

        let mut leaders_ids = vec![public_key];
        leaders_ids.append(&mut self.consensus_leader_ids.clone());
        genesis_model.blockchain_configuration.consensus_leader_ids = Some(leaders_ids.clone());
        genesis_model.blockchain_configuration.block0_consensus = self.block0_consensus.clone();
        genesis_model.blockchain_configuration.bft_slots_ratio = self.bft_slots_ratio.clone();
        genesis_model.blockchain_configuration.kes_update_speed = self.kes_update_speed.clone();

        if self.slots_per_epoch.is_some() {
            genesis_model.blockchain_configuration.slots_per_epoch = self.slots_per_epoch;
        }
        if self.slot_duration.is_some() {
            genesis_model.blockchain_configuration.slot_duration = self.slot_duration;
        }
        if self.epoch_stability_depth.is_some() {
            genesis_model.blockchain_configuration.epoch_stability_depth =
                self.epoch_stability_depth;
        }

        genesis_model
            .blockchain_configuration
            .consensus_genesis_praos_active_slot_coeff =
            self.consensus_genesis_praos_active_slot_coeff.clone();
        genesis_model.blockchain_configuration.linear_fees = self.linear_fees.clone();
        let certs = self.certs.iter().cloned().map(Initial::Cert);
        genesis_model.initial.extend(certs);
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
