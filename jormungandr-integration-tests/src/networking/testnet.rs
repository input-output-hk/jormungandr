#![cfg(feature = "testnet")]

use crate::{
    common::{
        configuration::jormungandr_config::JormungandrConfig,
        jcli_wrapper,
        jormungandr::{ConfigurationBuilder, JormungandrProcess, Starter, StartupVerificationMode},
        process_utils::WaitBuilder,
    },
    jormungandr::genesis::stake_pool::{create_new_stake_pool, delegate_stake, retire_stake_pool},
};
use jormungandr_lib::{interfaces::TrustedPeer, wallet::Wallet};
use std::env;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TestnetConfig {
    actor_account_private_key: String,
    block0_hash: String,
    public_ip: String,
    public_port: String,
    listen_port: String,
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
            .expect(&format!(
                "{} env is not set",
                actor_account_private_key_var_name
            ));

        let block_hash_var_name = format!("{}_BLOCK0_HASH", prefix);
        let block0_hash = env::var(block_hash_var_name.clone()).expect(&format!(
            "{} env is not set",
            actor_account_private_key_var_name
        ));

        let public_ip_var_name = "PUBLIC_IP";
        let public_ip = env::var(public_ip_var_name.clone())
            .expect(&format!("{} env is not set", public_ip_var_name));

        let public_port_var_name = "PUBLIC_PORT";
        let public_port = env::var(public_port_var_name.clone())
            .expect(&format!("{} env is not set", public_port_var_name));

        let listen_port_var_name = "LISTEN_PORT";
        let listen_port = env::var(listen_port_var_name.clone())
            .expect(&format!("{} env is not set", listen_port_var_name));

        let trusted_peers = Self::initialize_trusted_peers(prefix);

        TestnetConfig {
            actor_account_private_key,
            block0_hash,
            public_ip,
            public_port,
            listen_port,
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
                id: trusted_peer_id
                    .expect("incorrect trusted peer id")
                    .parse()
                    .expect("cannot parse trusted peer address"),
            });
        }
        trusted_peers
    }

    pub fn make_config(&self) -> JormungandrConfig {
        ConfigurationBuilder::new()
            .with_block_hash(self.block0_hash.to_string())
            .with_trusted_peers(self.trusted_peers.clone())
            .with_public_address(format!("/ip4/{}/tcp/{}", self.public_ip, self.public_port))
            .with_listen_address(format!("/ip4/0.0.0.0/tcp/{}", self.listen_port))
            .build()
    }

    pub fn block0_hash(&self) -> String {
        self.block0_hash.clone()
    }

    pub fn actor_account_private_key(&self) -> String {
        self.actor_account_private_key.clone()
    }
}

fn create_actor_account(private_key: &str, jormungandr: &JormungandrProcess) -> Wallet {
    let actor_account = Wallet::from_existing_account(&private_key, None);
    let account_state = jcli_wrapper::assert_rest_account_get_stats(
        &actor_account.address().to_string(),
        &jormungandr.rest_address(),
    );
    Wallet::from_existing_account(&private_key, Some(account_state.counter()))
}

#[test]
pub fn itn_bootstrap() {
    let testnet_config = TestnetConfig::new_itn();

    let _jormungandr = Starter::new()
        .config(testnet_config.make_config())
        .timeout(Duration::from_secs(4000))
        .benchmark("passive_node_itn_bootstrap")
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

#[test]
pub fn nightly_bootstrap() {
    let testnet_config = TestnetConfig::new_nightly();

    let _jormungandr = Starter::new()
        .config(testnet_config.make_config())
        .timeout(Duration::from_secs(4000))
        .benchmark("passive_node_nightly_bootstrap")
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

#[test]
pub fn qa_bootstrap() {
    let testnet_config = TestnetConfig::new_qa();

    let _jormungandr = Starter::new()
        .config(testnet_config.make_config())
        .timeout(Duration::from_secs(4000))
        .benchmark("passive_node_qa_bootstrap")
        .passive()
        .verify_by(StartupVerificationMode::Rest)
        .start()
        .unwrap();
}

#[test]
#[ignore]
pub fn e2e_stake_pool() {
    let testnet_config = TestnetConfig::new_qa();
    let block0_hash = testnet_config.block0_hash();

    let jormungandr = Starter::new()
        .config(testnet_config.make_config())
        .timeout(Duration::from_secs(4000))
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

    //register stake pool
    let stake_pool_id =
        create_new_stake_pool(&mut actor_account, &block0_hash, &jormungandr, &long_wait);
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        &block0_hash,
        &jormungandr,
        &long_wait,
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        &block0_hash,
        &jormungandr,
        &long_wait,
    );
}
