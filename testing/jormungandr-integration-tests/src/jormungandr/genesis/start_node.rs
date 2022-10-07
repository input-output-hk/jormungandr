use crate::startup;
use chain_crypto::Ed25519Extended;
use jormungandr_automation::{
    jormungandr::{Block0ConfigurationBuilder, NodeConfigBuilder},
    testing::keys,
};

#[test]
pub fn test_genesis_stake_pool_with_account_faucet_starts_successfully() {
    let faucet = thor::Wallet::default();
    let (_jormungandr, _) = startup::start_stake_pool(
        &[faucet],
        &[],
        Block0ConfigurationBuilder::default(),
        NodeConfigBuilder::default(),
    )
    .unwrap();
}

#[test]
pub fn test_genesis_stake_pool_with_utxo_faucet_starts_successfully() {
    let stake_key = keys::create_new_key_pair::<Ed25519Extended>();
    let faucet = startup::create_new_delegation_address_for(&stake_key.identifier());
    let (_jormungandr, _) = startup::start_stake_pool(
        &[faucet],
        &[],
        Block0ConfigurationBuilder::default(),
        NodeConfigBuilder::default(),
    )
    .unwrap();
}
