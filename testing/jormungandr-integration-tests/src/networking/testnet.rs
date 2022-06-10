use crate::jormungandr::genesis::stake_pool::{
    create_new_stake_pool, delegate_stake, retire_stake_pool,
};
use assert_fs::{fixture::PathChild, TempDir};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{
        download_last_n_releases, get_jormungandr_bin, ConfigurationBuilder, JormungandrParams,
        JormungandrProcess, Starter, StartupVerificationMode, Version,
    },
    testing::benchmark::storage_loading_benchmark_from_log,
};
use jormungandr_lib::interfaces::{BlockDate, Log, LogEntry, LogOutput, TrustedPeer};
use jortestkit::process::WaitBuilder;
use std::{env, path::PathBuf, time::Duration};
use thor::Wallet;

#[derive(Clone, Debug)]
pub struct TestnetConfig {
    actor_account_private_key: String,
    block0_hash: String,
    public_ip: String,
    public_port: String,
    trusted_peers: Vec<TrustedPeer>,
}

impl TestnetConfig {
    pub fn new_itn() -> Self {
        Self::new("ITN")
    }

    pub fn new_qa() -> Self {
        Self::new("QA")
    }

    pub fn new_nightly() -> Self {
        Self::new("NIGHTLY")
    }

    pub fn new(prefix: &str) -> Self {
        let actor_account_private_key_var_name = format!("{}_ACCOUNT_SK", prefix);
        let actor_account_private_key = env::var(actor_account_private_key_var_name.clone())
            .unwrap_or_else(|_| panic!("{} env is not set", actor_account_private_key_var_name));

        let block_hash_var_name = format!("{}_BLOCK0_HASH", prefix);
        let block0_hash = env::var(block_hash_var_name)
            .unwrap_or_else(|_| panic!("{} env is not set", actor_account_private_key_var_name));

        let public_ip_var_name = "PUBLIC_IP";
        let public_ip = env::var(public_ip_var_name)
            .unwrap_or_else(|_| panic!("{} env is not set", public_ip_var_name));

        let public_port_var_name = "PUBLIC_PORT";
        let public_port = env::var(public_port_var_name)
            .unwrap_or_else(|_| panic!("{} env is not set", public_port_var_name));

        let trusted_peers = Self::initialize_trusted_peers(prefix);

        TestnetConfig {
            actor_account_private_key,
            block0_hash,
            public_ip,
            public_port,
            trusted_peers,
        }
    }

    fn initialize_trusted_peers(prefix: &str) -> Vec<TrustedPeer> {
        let mut trusted_peers = Vec::new();
        for i in 1..10 {
            let trusted_peer_address = env::var(format!("{}_TRUSTED_PEER_{}_ADDRESS", prefix, i));
            let trusted_peer_id = env::var(format!("{}_TRUSTED_PEER_{}_ID", prefix, i));

            if trusted_peer_address.is_err() || trusted_peer_id.is_err() {
                break;
            }

            trusted_peers.push(TrustedPeer {
                address: trusted_peer_address
                    .expect("incorrect trusted peer address")
                    .parse()
                    .expect("cannot parse trusted peer address"),
                id: Some(
                    trusted_peer_id
                        .expect("incorrect trusted peer id")
                        .parse()
                        .expect("cannot parse trusted peer id"),
                ),
            });
        }
        trusted_peers
    }

    pub fn make_leader_config(&self, temp_dir: &TempDir) -> JormungandrParams {
        ConfigurationBuilder::new()
            .with_block_hash(self.block0_hash())
            .with_storage(&temp_dir.child("storage"))
            .with_log(Log(LogEntry {
                format: "json".to_string(),
                level: "info".to_string(),
                output: LogOutput::File(temp_dir.child("leader.log").path().to_path_buf()),
            }))
            .with_trusted_peers(self.trusted_peers.clone())
            .with_public_address(format!("/ip4/{}/tcp/{}", self.public_ip, self.public_port))
            .build(temp_dir)
    }

    pub fn make_passive_config(&self, temp_dir: &TempDir) -> JormungandrParams {
        ConfigurationBuilder::new()
            .with_block_hash(self.block0_hash())
            .with_storage(&temp_dir.child("storage"))
            .with_log(Log(LogEntry {
                format: "json".to_string(),
                level: "info".to_string(),
                output: LogOutput::File(temp_dir.child("passive.log").path().to_path_buf()),
            }))
            .with_trusted_peers(self.trusted_peers.clone())
            .build(temp_dir)
    }

    pub fn block0_hash(&self) -> String {
        self.block0_hash.clone()
    }

    pub fn actor_account_private_key(&self) -> String {
        self.actor_account_private_key.clone()
    }
}

fn create_actor_account(private_key: &str, jormungandr: &JormungandrProcess) -> Wallet {
    let jcli: JCli = Default::default();
    let discrimination = jormungandr.rest().settings().unwrap().discrimination;
    let actor_account = Wallet::from_existing_account(private_key, None, discrimination);
    let account_state = jcli
        .rest()
        .v0()
        .account_stats(actor_account.address().to_string(), jormungandr.rest_uri());
    Wallet::from_existing_account(
        private_key,
        Some(account_state.counters()[0].into()),
        discrimination,
    )
}

fn bootstrap_current(testnet_config: TestnetConfig, network_alias: &str) {
    let temp_dir = TempDir::new().unwrap();
    let mut jormungandr_config = testnet_config.make_passive_config(&temp_dir);

    // start from itn trusted peers
    let jormungandr_from_trusted_peers = Starter::new()
        .config(jormungandr_config.clone())
        .timeout(Duration::from_secs(96_000))
        .benchmark(&format!("passive_node_{}_bootstrap", network_alias))
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
    jormungandr_from_trusted_peers.shutdown();

    jormungandr_config.refresh_instance_params();

    // start from storage
    let loading_from_storage_timeout = Duration::from_secs(12_000);
    let jormungandr_from_storage = Starter::new()
        .config(jormungandr_config)
        .timeout(loading_from_storage_timeout)
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    storage_loading_benchmark_from_log(
        &jormungandr_from_storage.logger,
        &format!("passive_node_{}_loading_from_storage", network_alias),
        loading_from_storage_timeout,
    )
    .print();

    let config = ConfigurationBuilder::new()
        .with_block_hash(testnet_config.block0_hash())
        .with_trusted_peers(vec![jormungandr_from_storage.to_trusted_peer()])
        .build(&temp_dir);

    let _jormungandr_from_local_trusted_peer = Starter::new()
        .config(config)
        .timeout(Duration::from_secs(15_000))
        .benchmark(&format!(
            "passive_node_from_trusted_peer_{}_bootstrap",
            network_alias
        ))
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

fn bootstrap_legacy(testnet_config: TestnetConfig, network_prefix: &str) {
    let temp_dir = TempDir::new().unwrap();
    let (_, version) = get_legacy_app(&temp_dir);

    let mut legacy_jormungandr_config = testnet_config.make_passive_config(&temp_dir);

    // bootstrap node as legacy node
    let legacy_jormungandr = Starter::new()
        .config(legacy_jormungandr_config.clone())
        .timeout(Duration::from_secs(48_000))
        .legacy(version.clone())
        .benchmark(&format!(
            "legacy node bootstrap from {} trusted peers",
            network_prefix
        ))
        .passive()
        .start()
        .unwrap();

    let config = ConfigurationBuilder::new()
        .with_block_hash(testnet_config.block0_hash())
        .with_trusted_peers(vec![legacy_jormungandr.to_trusted_peer()])
        .build(&temp_dir);

    // bootstrap latest node from legacy node peer
    let new_jormungandr_from_local_trusted_peer = Starter::new()
        .config(config)
        .timeout(Duration::from_secs(24_000))
        .benchmark(&format!(
            "latest node bootstrap from {} legacy node",
            network_prefix
        ))
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    new_jormungandr_from_local_trusted_peer.shutdown();
    legacy_jormungandr.shutdown();

    // test node upgrade from old data
    legacy_jormungandr_config.refresh_instance_params();

    let latest_jormungandr = Starter::new()
        .config(legacy_jormungandr_config.clone())
        .timeout(Duration::from_secs(48_000))
        .benchmark(&format!(
            "latest node bootstrap from {} legacy data",
            network_prefix
        ))
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    latest_jormungandr.shutdown();

    // test rollback
    legacy_jormungandr_config.refresh_instance_params();

    let _rollback_jormungandr = Starter::new()
        .config(legacy_jormungandr_config)
        .timeout(Duration::from_secs(48_000))
        .legacy(version)
        .benchmark(&format!(
            "legacy node bootstrap from {} new data",
            network_prefix
        ))
        .passive()
        .start()
        .unwrap();
}

#[test]
pub fn itn_bootstrap_current() {
    bootstrap_current(TestnetConfig::new_itn(), "itn");
}

fn get_legacy_app(temp_dir: &TempDir) -> (PathBuf, Version) {
    let releases = download_last_n_releases(1);
    let last_release = releases.get(0).unwrap();
    let jormungandr = get_jormungandr_bin(last_release, temp_dir);
    (jormungandr, last_release.version())
}

#[test]
pub fn itn_bootstrap_legacy() {
    bootstrap_legacy(TestnetConfig::new_itn(), "itn");
}

#[ignore]
#[test]
pub fn itn_e2e_stake_pool() {
    e2e_stake_pool(TestnetConfig::new_itn());
}

#[test]
pub fn nightly_bootstrap_legacy() {
    bootstrap_current(TestnetConfig::new_nightly(), "nightly");
}

#[test]
pub fn nightly_bootstrap_current() {
    bootstrap_current(TestnetConfig::new_nightly(), "nightly")
}

#[test]
pub fn nightly_e2e_stake_pool() {
    e2e_stake_pool(TestnetConfig::new_nightly());
}

fn e2e_stake_pool(testnet_config: TestnetConfig) {
    let temp_dir = TempDir::new().unwrap();
    let block0_hash = testnet_config.block0_hash();

    let jormungandr = Starter::new()
        .config(testnet_config.make_leader_config(&temp_dir))
        .temp_dir(temp_dir)
        .timeout(Duration::from_secs(8000))
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();

    let mut actor_account =
        create_actor_account(&testnet_config.actor_account_private_key, &jormungandr);

    let long_wait = WaitBuilder::new()
        .tries(200)
        .sleep_between_tries(120)
        .build();

    let tx_expiry_date = BlockDate::new(1, 0);
    //register stake pool
    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        &block0_hash,
        tx_expiry_date,
        &jormungandr,
        &long_wait,
    );

    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        &block0_hash,
        tx_expiry_date,
        &jormungandr,
        &long_wait,
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        &block0_hash,
        tx_expiry_date,
        &jormungandr,
        &long_wait,
    );
}
