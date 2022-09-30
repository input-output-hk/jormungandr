use crate::jormungandr::{
    configuration, Block0ConfigurationBuilder, JormungandrParams, JormungandrProcess,
    NodeConfigBuilder, SecretModelFactory,
};
use assert_fs::fixture::{ChildPath, PathChild};
use chain_addr::Discrimination;
use chain_core::{packer::Codec, property::Serialize};
use chain_crypto::Ed25519;
use chain_impl_mockchain::{chaintypes::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{
        ActiveSlotCoefficient, Block0Configuration, BlockContentMaxSize, CommitteeIdDef,
        ConsensusLeaderId, Cors, EpochStabilityDepth, FeesGoTo, Initial, InitialToken, InitialUTxO,
        KesUpdateSpeed, Log, LogEntry, LogOutput, Mempool, NodeConfig, NodeSecret,
        NumberOfSlotsPerEpoch, Policy, ProposalExpiration, RewardParams, SignedCertificate,
        SlotDuration, TaxType, Tls, TrustedPeer, Value,
    },
};
use std::path::PathBuf;
const DEFAULT_SLOT_DURATION: u8 = 2;

#[derive(Clone, Debug)]
pub struct ConfigurationBuilder {
    funds: Vec<Initial>,
    tokens: Vec<Initial>,
    certs: Vec<Initial>,
    block0_hash: Option<String>,
    block0_consensus: ConsensusVersion,
    consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient,
    slots_per_epoch: NumberOfSlotsPerEpoch,
    slot_duration: SlotDuration,
    epoch_stability_depth: EpochStabilityDepth,
    kes_update_speed: KesUpdateSpeed,
    linear_fees: LinearFee,
    consensus_leader_ids: Vec<ConsensusLeaderId>,
    secret: Option<NodeSecret>,
    fees_go_to: Option<FeesGoTo>,
    total_reward_supply: Option<Value>,
    reward_parameters: Option<RewardParams>,
    treasury: Option<Value>,
    treasury_parameters: Option<TaxType>,
    node_config_builder: NodeConfigBuilder,
    rewards_history: bool,
    configure_default_log: bool,
    committee_ids: Vec<CommitteeIdDef>,
    leader_key_pair: KeyPair<Ed25519>,
    discrimination: Discrimination,
    block_content_max_size: BlockContentMaxSize,
    proposal_expiry_epochs: ProposalExpiration,
    tx_max_expiry_epochs: Option<u8>,
    log_level: String,
}

impl Default for ConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurationBuilder {
    pub fn new() -> Self {
        ConfigurationBuilder {
            funds: vec![],
            tokens: vec![],
            certs: vec![],
            consensus_leader_ids: vec![],
            secret: None,
            block0_hash: None,
            block0_consensus: ConsensusVersion::Bft,
            slots_per_epoch: NumberOfSlotsPerEpoch::new(100).unwrap(),
            slot_duration: SlotDuration::new(DEFAULT_SLOT_DURATION).unwrap(),
            epoch_stability_depth: 2600u32.into(),
            linear_fees: LinearFee::new(0, 0, 0),
            consensus_genesis_praos_active_slot_coeff: ActiveSlotCoefficient::MAXIMUM,
            kes_update_speed: KesUpdateSpeed::new(12 * 3600).unwrap(),
            node_config_builder: NodeConfigBuilder::new(),
            rewards_history: false,
            configure_default_log: true,
            committee_ids: vec![],
            leader_key_pair: KeyPair::generate(&mut rand::thread_rng()),
            proposal_expiry_epochs: Default::default(),
            fees_go_to: None,
            treasury: None,
            treasury_parameters: None,
            total_reward_supply: None,
            reward_parameters: None,
            discrimination: Discrimination::Test,
            block_content_max_size: 4092.into(),
            tx_max_expiry_epochs: None,
            log_level: "trace".into(),
        }
    }

    pub fn with_committees(&mut self, committees: &[CommitteeIdDef]) -> &mut Self {
        self.committee_ids = committees.to_vec();
        self
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

    pub fn with_kes_update_speed(&mut self, kes_update_speed: KesUpdateSpeed) -> &mut Self {
        self.kes_update_speed = kes_update_speed;
        self
    }

    pub fn with_proposal_expiry_epochs(&mut self, proposal_expiry_epochs: u32) -> &mut Self {
        self.proposal_expiry_epochs = ProposalExpiration::from(proposal_expiry_epochs);
        self
    }

    pub fn with_rewards_history(&mut self) -> &mut Self {
        self.rewards_history = true;
        self
    }

    pub fn with_committee_ids(&mut self, committee_ids: Vec<CommitteeIdDef>) -> &mut Self {
        self.committee_ids = committee_ids;
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
        self.consensus_leader_ids = consensus_leader_ids;
        self
    }

    pub fn with_initial_certs(&mut self, certs: Vec<SignedCertificate>) -> &mut Self {
        self.certs
            .extend(certs.iter().map(|cert| Initial::Cert(cert.clone())));
        self
    }

    pub fn with_fees_go_to(&mut self, fees_go_to: FeesGoTo) -> &mut Self {
        self.fees_go_to = Some(fees_go_to);
        self
    }

    pub fn with_rest_tls_config(&mut self, tls: Tls) -> &mut Self {
        self.node_config_builder.with_rest_tls_config(tls);
        self
    }

    pub fn with_rest_cors_config(&mut self, cors: Cors) -> &mut Self {
        self.node_config_builder.with_rest_cors_config(cors);
        self
    }

    pub fn with_storage(&mut self, temp_dir: &ChildPath) -> &mut Self {
        self.node_config_builder
            .with_storage(temp_dir.path().into());
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

    pub fn with_block_content_max_size(
        &mut self,
        block_content_max_size: BlockContentMaxSize,
    ) -> &mut Self {
        self.block_content_max_size = block_content_max_size;
        self
    }

    pub fn with_funds(&mut self, initial: Vec<InitialUTxO>) -> &mut Self {
        self.funds.push(Initial::Fund(initial));
        self
    }

    pub fn with_fund(&mut self, initial: InitialUTxO) -> &mut Self {
        self.funds.push(Initial::Fund(vec![initial]));
        self
    }

    pub fn with_funds_split_if_needed(&mut self, initials: Vec<InitialUTxO>) -> &mut Self {
        for chunks in initials.chunks(254) {
            self.with_funds(chunks.to_vec());
        }
        self
    }

    pub fn with_certs(&mut self, initial: Vec<SignedCertificate>) -> &mut Self {
        self.certs
            .extend(initial.iter().cloned().map(Initial::Cert));
        self
    }

    pub fn with_token(&mut self, token: InitialToken) -> &mut Self {
        self.tokens.push(Initial::Token(token));
        self
    }

    pub fn with_log(&mut self, log: Log) -> &mut Self {
        self.node_config_builder.with_log(log);
        self
    }

    pub fn with_log_path(&mut self, path: PathBuf, level: String) -> &mut Self {
        self.with_log(Log(LogEntry {
            format: "json".to_string(),
            level,
            output: LogOutput::File(path),
        }))
    }

    pub fn with_log_level(&mut self, level: String) -> &mut Self {
        self.log_level = level;
        self
    }

    pub fn without_log(&mut self) -> &mut Self {
        self.configure_default_log = false;
        self
    }

    pub fn with_policy(&mut self, policy: Policy) -> &mut Self {
        self.node_config_builder.with_policy(policy);
        self
    }

    pub fn with_mempool(&mut self, mempool: Mempool) -> &mut Self {
        self.node_config_builder.with_mempool(mempool);
        self
    }

    pub fn with_trusted_peers(&mut self, trusted_peers: Vec<TrustedPeer>) -> &mut Self {
        self.node_config_builder.with_trusted_peers(trusted_peers);
        self
    }

    pub fn with_trusted_peer(&mut self, node: &JormungandrProcess) -> &mut Self {
        self.node_config_builder
            .with_trusted_peers(vec![node.to_trusted_peer()]);
        self
    }

    pub fn with_public_address(&mut self, public_address: String) -> &mut Self {
        self.node_config_builder.with_public_address(public_address);
        self
    }

    pub fn with_listen_address(&mut self, listen_address: String) -> &mut Self {
        self.node_config_builder
            .with_listen_address(listen_address.parse().unwrap());
        self
    }

    pub fn with_block_hash(&mut self, block0_hash: impl Into<String>) -> &mut Self {
        self.block0_hash = Some(block0_hash.into());
        self
    }

    pub fn with_secret(&mut self, secret: NodeSecret) -> &mut Self {
        self.secret = Some(secret);
        self
    }

    pub fn with_leader_key_pair(&mut self, leader_key_pair: KeyPair<Ed25519>) -> &mut Self {
        self.leader_key_pair = leader_key_pair;
        self
    }

    pub fn with_treasury(&mut self, treasury: Value) -> &mut Self {
        self.treasury = Some(treasury);
        self
    }

    pub fn with_treasury_parameters(&mut self, treasury_parameters: TaxType) -> &mut Self {
        self.treasury_parameters = Some(treasury_parameters);
        self
    }

    pub fn with_reward_parameters(&mut self, reward_parameters: RewardParams) -> &mut Self {
        self.reward_parameters = Some(reward_parameters);
        self
    }

    pub fn with_total_rewards_supply(&mut self, total_reward_supply: Value) -> &mut Self {
        self.total_reward_supply = Some(total_reward_supply);
        self
    }

    pub fn with_discrimination(&mut self, discrimination: Discrimination) -> &mut Self {
        self.discrimination = discrimination;
        self
    }

    pub fn with_tx_max_expiry_epochs(&mut self, tx_max_expiry_epochs: u8) -> &mut Self {
        self.tx_max_expiry_epochs = Some(tx_max_expiry_epochs);
        self
    }

    pub fn build_block0(&self) -> Block0Configuration {
        let mut leaders_ids = self.consensus_leader_ids.clone();
        leaders_ids.push(self.leader_key_pair.identifier().into());

        let mut initial: Vec<Initial> = Vec::new();
        initial.extend(self.funds.iter().cloned());
        initial.extend(self.tokens.iter().cloned());
        initial.extend(self.certs.iter().cloned());

        let mut block0_config_builder = Block0ConfigurationBuilder::new();

        if let Some(tx_max_expiry_epochs) = self.tx_max_expiry_epochs {
            block0_config_builder = block0_config_builder
                .with_tx_max_expiry_epochs(tx_max_expiry_epochs)
                .to_owned();
        }

        if let Some(treasury_parameters) = self.treasury_parameters {
            block0_config_builder = block0_config_builder
                .with_treasury_parameters(Some(treasury_parameters))
                .to_owned();
        }

        if let Some(reward_parameters) = self.reward_parameters {
            block0_config_builder = block0_config_builder
                .with_reward_parameters(Some(reward_parameters))
                .to_owned();
        }

        block0_config_builder
            .with_discrimination(self.discrimination)
            .with_initial(initial)
            .with_leaders(leaders_ids)
            .with_block0_consensus(self.block0_consensus)
            .with_kes_update_speed(self.kes_update_speed)
            .with_slots_per_epoch(self.slots_per_epoch)
            .with_slot_duration(self.slot_duration)
            .with_fees_go_to(self.fees_go_to)
            .with_treasury(self.treasury)
            .with_epoch_stability_depth(self.epoch_stability_depth)
            .with_active_slot_coeff(self.consensus_genesis_praos_active_slot_coeff)
            .with_linear_fees(self.linear_fees.clone())
            .with_proposal_expiration(self.proposal_expiry_epochs)
            .with_block_content_max_size(self.block_content_max_size)
            .with_committee_ids(self.committee_ids.clone())
            .with_total_rewards_supply(self.total_reward_supply)
            .build()
    }

    pub fn build(&self, temp_dir: &impl PathChild) -> JormungandrParams<NodeConfig> {
        let mut node_config = self.node_config_builder.build();

        //remove id from trusted peers
        for trusted_peer in node_config.p2p.trusted_peers.iter_mut() {
            trusted_peer.id = None;
        }

        let block0_config = self.build_block0();
        let default_log_file = || temp_dir.child("node.log").path().to_path_buf();

        match (&node_config.log, self.configure_default_log) {
            (Some(log), _) => log.file_path().map_or_else(default_log_file, Into::into),
            (None, false) => default_log_file(),
            (None, true) => {
                let path = default_log_file();
                node_config.log = Some(Log(LogEntry {
                    level: "trace".to_string(),
                    format: "json".to_string(),
                    output: LogOutput::Stdout,
                }));
                path
            }
        };

        let genesis_block_hash = match self.block0_hash {
            Some(ref value) => value.clone(),
            None => block0_config.to_block().header().hash().to_string(),
        };

        let path_to_output_block = temp_dir.child("block0.bin");
        let file = std::fs::File::create(path_to_output_block.path()).unwrap();
        block0_config
            .to_block()
            .serialize(&mut Codec::new(file))
            .unwrap();

        fn write_secret(secret: &NodeSecret, output_file: ChildPath) -> PathBuf {
            configuration::write_secret(secret, &output_file);
            output_file.path().to_path_buf()
        }

        let secret_model_path = {
            let secret = self
                .secret
                .clone()
                .unwrap_or_else(|| SecretModelFactory::bft(self.leader_key_pair.signing_key()));
            let output_file = temp_dir.child("node_secret.yaml");
            write_secret(&secret, output_file)
        };

        let config_file = temp_dir.child("node_config.yaml");

        let params = JormungandrParams::new(
            node_config,
            config_file.path(),
            path_to_output_block.path(),
            genesis_block_hash,
            secret_model_path,
            block0_config,
            self.rewards_history,
        );

        params.write_node_config();
        params
    }
}
