#![cfg(feature = "testnet")]

use crate::{
    common::{
        configuration::genesis_model::{Fund, LinearFees},
        data::address::Account,
        file_utils,
        jcli_wrapper::{
            self, certificate::wrapper::JCLICertificateWrapper,
            jcli_transaction_wrapper::JCLITransactionWrapper,
        },
        jormungandr::starter::start_jormungandr_node_as_passive_with_timeout,
        process_utils, startup,
    },
    jormungandr::genesis::stake_pool::{create_new_stake_pool, delegate_stake, retire_stake_pool},
};

use chain_addr::Discrimination;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Certificate, Value},
};
use std::str::FromStr;
use std::time::SystemTime;

fn create_account_from_secret_key(private_key: String) -> Account {
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    Account::new(&private_key, &public_key, &address)
}

use std::env;

#[test]
pub fn e2e_stake_pool() {
    let actor_account_private_key = env::var("ACCOUNT_SK");
    let block0_hash = env::var("BLOCK0_HASH");
    let public_address = env::var("PUBLIC_ADDRESS");
    let listen_address = env::var("LISTEN_ADDRESS");

    if actor_account_private_key.is_err()
        || block0_hash.is_err()
        || public_address.is_err()
        || listen_address.is_err()
    {
        panic!("Test requires environment variables to be set: [ACCOUNT_SK,BLOCK0_HASH,PUBLIC_ADDRESS,LISTEN_ADDRESS]");
    }

    let block0_hash = block0_hash.unwrap();
    let public_address = public_address.unwrap();
    let listen_address = listen_address.unwrap();
    let mut actor_account =
        create_account_from_secret_key(actor_account_private_key.unwrap().to_string());

    let trusted_peers = vec![
        "/ip4/3.123.177.192/tcp/3000".to_owned(),
        "/ip4/52.57.157.167/tcp/3000".to_owned(),
        "/ip4/3.123.155.47/tcp/3000".to_owned(),
        "/ip4/3.115.57.216/tcp/3000".to_owned(),
        "/ip4/3.112.185.217/tcp/3000".to_owned(),
        "/ip4/18.139.40.4/tcp/3000".to_owned(),
        "/ip4/18.140.134.230/tcp/3000".to_owned(),
    ];

    let mut config = startup::ConfigurationBuilder::new()
        .with_block_hash(block0_hash.to_string())
        .with_trusted_peers(trusted_peers)
        .with_public_address(public_address.to_string())
        .with_listen_address(listen_address.to_string())
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let jormungandr = start_jormungandr_node_as_passive_with_timeout(&mut config, 120, 6);

    //register stake pool
    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        "1234",
        &block0_hash,
        &jormungandr_rest_address,
    );
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        &block0_hash,
        &jormungandr_rest_address,
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        &block0_hash,
        &jormungandr_rest_address,
    );
}
