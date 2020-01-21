use crate::common::{
    configuration::{
        jormungandr_config::JormungandrConfig,
        node_config_model::{Log, LogEntry, NodeConfig, TrustedPeer},
        secret_model::SecretModel,
        Block0ConfigurationBuilder,
    },
    file_utils, jcli_wrapper,
    startup::build_genesis_block,
};
use chain_impl_mockchain::{block::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, Block0Configuration, ConsensusLeaderId, EpochStabilityDepth, Initial,
    InitialUTxO, KESUpdateSpeed, Mempool, NumberOfSlotsPerEpoch, SignedCertificate, SlotDuration,
};

use std::path::PathBuf;

pub struct ConfigurationBuilder {
    funds: Vec<Initial>,
    certs: Vec<Initial>,
    trusted_peers: Option<Vec<TrustedPeer>>,
    public_address: Option<String>,
    listen_address: Option<String>,
    block0_hash: Option<String>,
    block0_consensus: ConsensusVersion,
    log: Option<Log>,
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    slots_per_epoch: NumberOfSlotsPerEpoch,
    slot_duration: SlotDuration,
    epoch_stability_depth: EpochStabilityDepth,
    kes_update_speed: KESUpdateSpeed,
    linear_fees: LinearFee,
    consensus_leader_ids: Vec<ConsensusLeaderId>,
    mempool: Option<Mempool>,
    enable_explorer: bool,
}

impl ConfigurationBuilder {
    pub fn new() -> Self {
        ConfigurationBuilder {
            funds: vec![],
            certs: vec![],
            consensus_leader_ids: vec![],
            trusted_peers: None,
            listen_address: None,
            public_address: None,
            block0_hash: None,
            block0_consensus: ConsensusVersion::Bft,
            slots_per_epoch: NumberOfSlotsPerEpoch::new(100).unwrap(),
            slot_duration: SlotDuration::new(1).unwrap(),
            epoch_stability_depth: 2600u32.into(),
            log: Some(Log(vec![LogEntry {
                level: Some("info".to_string()),
                format: Some("json".to_string()),
            }])),
            linear_fees: LinearFee::new(0, 0, 0),
            consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::MAXIMUM,
            kes_update_speed: KESUpdateSpeed::new(12 * 3600).unwrap(),
            mempool: None,
            enable_explorer: false,
        }
    }

    pub fn with_slots_per_epoch(&mut self, slots_per_epoch: u32) -> &mut Self {
        self.slots_per_epoch = NumberOfSlotsPerEpoch::new(slots_per_epoch).unwrap();
        self
    }

    pub fn with_slot_duration(&mut self, slot_duration: u8) -> &mut Self {
        self.slot_duration = SlotDuration::new(slot_duration).unwrap();
        self
    }

    pub fn with_epoch_stability_depth(&mut self, epoch_stability_depth: u32) -> &mut Self {
        self.epoch_stability_depth = epoch_stability_depth.into();
        self
    }

    pub fn with_kes_update_speed(&mut self, kes_update_speed: KESUpdateSpeed) -> &mut Self {
        self.kes_update_speed = kes_update_speed;
        self
    }

    pub fn with_linear_fees(&mut self, linear_fees: LinearFee) -> &mut Self {
        self.linear_fees = linear_fees;
        self
    }

    pub fn with_consensus_leaders_ids(
        &mut self,
        consensus_leader_ids: Vec<ConsensusLeaderId>,
    ) -> &mut Self {
        self.consensus_leader_ids = consensus_leader_ids.clone();
        self
    }

    pub fn with_initial_certs(&mut self, certs: Vec<String>) -> &mut Self {
        self.certs.extend(certs.iter().map(|cert| {
            let signed_cert: SignedCertificate = cert.parse().unwrap();
            Initial::Cert(signed_cert)
        }));
        self
    }

    pub fn with_explorer(&mut self) -> &mut Self {
        self.enable_explorer = true;
        self
    }

    pub fn with_block0_consensus(&mut self, consensus: ConsensusVersion) -> &mut Self {
        self.block0_consensus = consensus;
        self
    }

    pub fn with_consensus_genesis_praos_active_slot_coeff(
        &mut self,
        active_slot_coeff: ActiveSlotCoefficient,
    ) -> &mut Self {
        self.consensus_genesis_praos_active_slot_coeff = active_slot_coeff;
        self
    }

    pub fn with_funds(&mut self, initial: Vec<InitialUTxO>) -> &mut Self {
        self.funds.push(Initial::Fund(initial));
        self
    }

    pub fn with_certs(&mut self, initial: Vec<SignedCertificate>) -> &mut Self {
        self.certs
            .extend(initial.iter().cloned().map(Initial::Cert));
        self
    }

    pub fn with_log(&mut self, log: Log) -> &mut Self {
        self.log = Some(log.clone());
        self
    }

    pub fn with_trusted_peers(&mut self, trusted_peers: Vec<TrustedPeer>) -> &mut Self {
        self.trusted_peers = Some(trusted_peers.clone());
        self
    }

    pub fn with_public_address(&mut self, public_address: String) -> &mut Self {
        self.public_address = Some(public_address.clone());
        self
    }

    pub fn with_listen_address(&mut self, listen_address: String) -> &mut Self {
        self.listen_address = Some(listen_address.clone());
        self
    }

    pub fn with_block_hash(&mut self, block0_hash: String) -> &mut Self {
        self.block0_hash = Some(block0_hash.clone());
        self
    }

    pub fn with_mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.mempool = Some(mempool);
        self
    }

    pub fn serialize(block0_configuration: &Block0Configuration) -> PathBuf {
        let content = serde_yaml::to_string(&block0_configuration).unwrap();
        let input_yaml_file_path = file_utils::create_file_in_temp("genesis.yaml", &content);
        input_yaml_file_path
    }

    pub fn build(&self) -> JormungandrConfig {
        let mut node_config = NodeConfig::new();

        if let Some(listen_address) = &self.listen_address {
            node_config.p2p.listen_address = listen_address.to_string();
        }
        if let Some(public_address) = &self.public_address {
            node_config.p2p.public_address = public_address.to_string();
        }
        if let Some(mempool) = &self.mempool {
            node_config.mempool = mempool.clone();
        }

        node_config.p2p.trusted_peers = self.trusted_peers.clone();
        node_config.log = self.log.clone();
        node_config.explorer.enabled = self.enable_explorer;

        let node_config_path = NodeConfig::serialize(&node_config);

        let secret_key = jcli_wrapper::assert_key_generate("ed25519");
        let public_key = jcli_wrapper::assert_key_to_public_default(&secret_key);

        let mut leaders_ids = self.consensus_leader_ids.clone();
        leaders_ids.push(serde_yaml::from_str(&public_key).unwrap());

        let mut initial: Vec<Initial> = Vec::new();
        initial.extend(self.funds.iter().cloned());
        initial.extend(self.certs.iter().cloned());

        let block0_config = Block0ConfigurationBuilder::new()
            .with_initial(initial)
            .with_leaders(leaders_ids)
            .with_block0_consensus(self.block0_consensus.clone())
            .with_kes_update_speed(self.kes_update_speed.clone())
            .with_slots_per_epoch(self.slots_per_epoch)
            .with_slot_duration(self.slot_duration)
            .with_epoch_stability_depth(self.epoch_stability_depth)
            .with_active_slot_coeff(self.consensus_genesis_praos_active_slot_coeff.clone())
            .with_linear_fees(self.linear_fees.clone())
            .build();

        let path_to_output_block = build_genesis_block(&block0_config);

        let mut config = JormungandrConfig::from(block0_config, node_config);

        let secret_model = SecretModel::new_bft(&secret_key);
        let secret_model_path = SecretModel::serialize(&secret_model);

        config.secret_models = vec![secret_model];
        config.secret_model_paths = vec![secret_model_path];
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
