use crate::{
    testing::{
        configuration,
        jcli::JCli,
        jormungandr::JormungandrProcess,
        startup::{build_genesis_block, create_new_key_pair},
        Block0ConfigurationBuilder, JormungandrParams, NodeConfigBuilder, SecretModelFactory,
    },
    wallet::Wallet,
};
use assert_fs::fixture::{ChildPath, PathChild};
use chain_addr::Discrimination;
use chain_crypto::Ed25519;
use chain_impl_mockchain::{chaintypes::ConsensusVersion, fee::LinearFee};
use jormungandr_lib::crypto::key::KeyPair;
use jormungandr_lib::interfaces::Block0Configuration;
use jormungandr_lib::interfaces::{
    ActiveSlotCoefficient, CommitteeIdDef, ConsensusLeaderId, Cors, EpochStabilityDepth, FeesGoTo,
    Initial, InitialUTxO, KesUpdateSpeed, Log, LogEntry, LogOutput, Mempool, NodeConfig,
    NodeSecret, NumberOfSlotsPerEpoch, Policy, SignedCertificate, SlotDuration, Tls, TrustedPeer,
    Value,
};
use std::path::PathBuf;
const DEFAULT_SLOT_DURATION: u8 = 2;

#[derive(Clone, Debug)]
pub struct ConfigurationBuilder {
    funds: Vec<Initial>,
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
    treasury: Option<Value>,
    node_config_builder: NodeConfigBuilder,
    rewards_history: bool,
    block_content_max_size: u32,
    configure_default_log: bool,
    committee_ids: Vec<CommitteeIdDef>,
    leader_key_pair: KeyPair<Ed25519>,
    discrimination: Discrimination,
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
            block_content_max_size: 4096,
            rewards_history: false,
            configure_default_log: true,
            committee_ids: vec![],
            leader_key_pair: create_new_key_pair::<Ed25519>(),
            fees_go_to: None,
            treasury: None,
            total_reward_supply: None,
            discrimination: Discrimination::Test,
            tx_max_expiry_epochs: None,
            log_level: "trace".into(),
        }
    }

    pub fn with_committees(&mut self, wallets: &[&Wallet]) -> &mut Self {
        self.committee_ids = wallets.iter().map(|w| w.to_committee_id()).collect();
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

    pub fn with_explorer(&mut self) -> &mut Self {
        self.node_config_builder.with_explorer();
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

    pub fn with_block_content_max_size(&mut self, block_content_max_size: u32) -> &mut Self {
        self.block_content_max_size = block_content_max_size;
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

    pub fn with_funds(&mut self, initial: Vec<InitialUTxO>) -> &mut Self {
        self.funds.push(Initial::Fund(initial));
        self
    }

    pub fn with_fund(&mut self, initial: InitialUTxO) -> &mut Self {
        self.funds.push(Initial::Fund(vec![initial]));
        self
    }

    pub fn with_funds_split_if_needed(&mut self, initials: Vec<InitialUTxO>) -> &mut Self {
        let mut funds = Vec::new();
        for initial in initials.iter() {
            funds.push(initial.clone());

            if funds.len() >= 254 {
                self.with_funds(funds.clone());
                funds.clear();
            }
        }
        self
    }

    pub fn with_certs(&mut self, initial: Vec<SignedCertificate>) -> &mut Self {
        self.certs
            .extend(initial.iter().cloned().map(Initial::Cert));
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

    pub fn build(&self, temp_dir: &impl PathChild) -> JormungandrParams<NodeConfig> {
        let mut node_config = self.node_config_builder.build();

        //remove id from trusted peers
        for trusted_peer in node_config.p2p.trusted_peers.iter_mut() {
            trusted_peer.id = None;
        }

        let default_log_file = || temp_dir.child("node.log").path().to_path_buf();

        let log_file_path = match (&node_config.log, self.configure_default_log) {
            (Some(log), _) => log.file_path().map_or_else(default_log_file, Into::into),
            (None, false) => default_log_file(),
            (None, true) => {
                let path = default_log_file();
                node_config.log = Some(Log(LogEntry {
                    level: self.log_level.clone(),
                    format: "json".to_string(),
                    output: LogOutput::Stdout,
                }));
                path
            }
        };

        let leader_key_pair = self
            .leader_key_pair
            .clone()
            .unwrap_or_else(create_new_key_pair::<Ed25519>);
        let mut leaders_ids = self.consensus_leader_ids.clone();
        leaders_ids.push(self.leader_key_pair.identifier().into());

        let mut initial: Vec<Initial> = Vec::new();
        initial.extend(self.funds.iter().cloned());
        initial.extend(self.certs.iter().cloned());

        let mut block0_config_builder = Block0ConfigurationBuilder::new();

        if let Some(tx_max_expiry_epochs) = self.tx_max_expiry_epochs {
            block0_config_builder = block0_config_builder
                .with_tx_max_expiry_epochs(tx_max_expiry_epochs)
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
            .with_linear_fees(self.linear_fees)
            .with_block_content_max_size(self.block_content_max_size.into())
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

        let default_log_file = || temp_dir.child("node.log").path().to_path_buf();

        let log_file_path = match (&node_config.log, self.configure_default_log) {
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

        let block0_config = self.build_block0();

        let path_to_output_block = build_genesis_block(&block0_config, temp_dir);
        let genesis_block_hash = match self.block0_hash {
            Some(ref value) => value.clone(),
            None => {
                let jcli: JCli = Default::default();
                jcli.genesis().hash(&path_to_output_block).to_string()
            }
        };

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
            path_to_output_block,
            genesis_block_hash,
            secret_model_path,
            block0_config,
            self.rewards_history,
            log_file_path,
        );

        params.write_node_config();
        params
    }
}
