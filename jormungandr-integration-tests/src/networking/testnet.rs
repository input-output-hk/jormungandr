#![cfg(feature = "testnet")]

use crate::{
    common::{
        configuration::{jormungandr_config::JormungandrConfig, node_config_model::TrustedPeer},
        data::address::Account,
        jcli_wrapper,
        jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode},
        process_utils::WaitBuilder,
    },
    jormungandr::genesis::stake_pool::{create_new_stake_pool, delegate_stake, retire_stake_pool},
};
use chain_addr::Discrimination;
use std::env;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TestnetConfig {
    actor_account: Account,
    block0_hash: String,
    public_ip: String,
    public_port: String,
    trusted_peers: Vec<TrustedPeer>,
}

impl TestnetConfig {
    pub fn new() -> Self {
        let actor_account_private_key = env::var("ACCOUNT_SK").expect("ACCOUNT_SK env is not set");
        let block0_hash = env::var("BLOCK0_HASH").expect("BLOCK0_HASH env is not set");
        let public_ip = env::var("PUBLIC_IP").expect("PUBLIC_IP env is not set");
        let public_port = env::var("PUBLIC_PORT").expect("PUBLIC_PORT env is not set");
        let actor_account =
            Self::create_account_from_secret_key(actor_account_private_key.to_string());
        let trusted_peers = Self::initialize_trusted_peers();

        TestnetConfig {
            actor_account,
            block0_hash,
            public_ip,
            public_port,
            trusted_peers,
        }
    }

    fn create_account_from_secret_key(private_key: String) -> Account {
        let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
        let address = jcli_wrapper::assert_address_account(&public_key, Discrimination::Test);
        Account::new(&private_key, &public_key, &address)
    }

    fn initialize_trusted_peers() -> Vec<TrustedPeer> {
        let mut trusted_peers = Vec::new();
        for i in 1..10 {
            let trusted_peer_address = env::var(format!("TRUSTED_PEER_{}_ADDRESS", i));
            let trusted_peer_id = env::var(format!("TRUSTED_PEER_{}_ID", i));

            if trusted_peer_address.is_err() || trusted_peer_id.is_err() {
                break;
            }

            trusted_peers.push(TrustedPeer {
                address: trusted_peer_address.unwrap(),
                id: trusted_peer_id.unwrap(),
            });
        }
        trusted_peers
    }

    pub fn make_config(&self) -> JormungandrConfig {
        ConfigurationBuilder::new()
            .with_block_hash(self.block0_hash.to_string())
            .with_trusted_peers(self.trusted_peers.clone())
            .with_public_address(format!("/ip4/{}/tcp/{}", self.public_ip, self.public_port))
            .with_listen_address(format!("/ip4/0.0.0.0/tcp/{}", self.public_port))
            .build()
    }

    pub fn block0_hash(&self) -> String {
        self.block0_hash.clone()
    }

    pub fn actor_account(&self) -> Account {
        self.actor_account.clone()
    }
}

#[test]
pub fn e2e_stake_pool() {
    let testnet_config = TestnetConfig::new();
    let mut actor_account = testnet_config.actor_account();
    let block0_hash = testnet_config.block0_hash();

    let jormungandr = Starter::new()
        .config(testnet_config.make_config())
        .timeout(Duration::from_secs(1000))
        .passive()
        .verify_by(StartupVerificationMode::Log)
        .start()
        .unwrap();

    let long_wait = WaitBuilder::new()
        .tries(100)
        .sleep_between_tries(120)
        .build();

    //register stake pool
    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        "1234",
        &block0_hash,
        &jormungandr.rest_address(),
        &long_wait,
    );
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        &block0_hash,
        &jormungandr.rest_address(),
        &long_wait,
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        &block0_hash,
        &jormungandr.rest_address(),
        &long_wait,
    );
}
