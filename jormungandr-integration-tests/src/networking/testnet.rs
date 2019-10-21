#![cfg(feature = "testnet")]

use crate::{
    common::{
        configuration::{
            genesis_model::{Fund, LinearFees},
            node_config_model::TrustedPeer,
        },
        data::address::Account,
        file_utils,
        jcli_wrapper::{
            self, certificate::wrapper::JCLICertificateWrapper,
            jcli_transaction_wrapper::JCLITransactionWrapper,
        },
        jormungandr::{ConfigurationBuilder, Starter, StartupVerificationMode},
        process_utils, startup,
    },
    jormungandr::genesis::stake_pool::{create_new_stake_pool, delegate_stake, retire_stake_pool},
};

use chain_addr::Discrimination;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Certificate, Value},
};
use std::{env, str::FromStr, time::SystemTime};

fn create_account_from_secret_key(private_key: String) -> Account {
    let public_key = jcli_wrapper::assert_key_to_public_default(&private_key);
    let address = jcli_wrapper::assert_address_single(&public_key, Discrimination::Test);
    Account::new(&private_key, &public_key, &address)
}

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
        TrustedPeer {
            address: "/ip4/3.115.194.22/tcp/3000".to_string(),
            id: "ed25519_pk1npsal4j9p9nlfs0fsmfjyga9uqk5gcslyuvxy6pexxr0j34j83rsf98wl2".to_string(),
        },
        TrustedPeer {
            address: "/ip4/13.113.10.64/tcp/3000".to_string(),
            id: "ed25519_pk16pw2st5wgx4558c6temj8tzv0pqc37qqjpy53fstdyzwxaypveys3qcpfl".to_string(),
        },
        TrustedPeer {
            address: "/ip4/52.57.214.174/tcp/3000".to_string(),
            id: "ed25519_pk1v4cj0edgmp8f2m5gex85jglrs2ruvu4z7xgy8fvhr0ma2lmyhtyszxtejz".to_string(),
        },
        TrustedPeer {
            address: "/ip4/3.120.96.93/tcp/3000".to_string(),
            id: "ed25519_pk10gmg0zkxpuzkghxc39n3a646pdru6xc24rch987cgw7zq5pmytmszjdmvh".to_string(),
        },
        TrustedPeer {
            address: "/ip4/52.28.134.8/tcp/3000".to_string(),
            id: "ed25519_pk1unu66eej6h6uxv4j4e9crfarnm6jknmtx9eknvq5vzsqpq6a9vxqr78xrw".to_string(),
        },
        TrustedPeer {
            address: "/ip4/13.52.208.132/tcp/3000".to_string(),
            id: "ed25519_pk15ppd5xlg6tylamskqkxh4rzum26w9acph8gzg86w4dd9a88qpjms26g5q9".to_string(),
        },
        TrustedPeer {
            address: "/ip4/54.153.19.202/tcp/3000".to_string(),
            id: "ed25519_pk1j9nj2u0amlg28k27pw24hre0vtyp3ge0xhq6h9mxwqeur48u463s0crpfk".to_string(),
        },
    ];

    let config = ConfigurationBuilder::new()
        .with_block_hash(block0_hash.to_string())
        .with_trusted_peers(trusted_peers)
        .with_public_address(public_address.to_string())
        .with_listen_address(listen_address.to_string())
        .build();

    let jormungandr = Starter::new()
        .config(config)
        .verify_by(StartupVerificationMode::Log)
        .start()
        .unwrap();

    //register stake pool
    let stake_pool_id = create_new_stake_pool(
        &mut actor_account,
        "1234",
        &block0_hash,
        &jormungandr.rest_address(),
    );
    delegate_stake(
        &mut actor_account,
        &stake_pool_id,
        &block0_hash,
        &jormungandr.rest_address(),
    );
    retire_stake_pool(
        &stake_pool_id,
        &mut actor_account,
        &block0_hash,
        &jormungandr.rest_address(),
    );
}
