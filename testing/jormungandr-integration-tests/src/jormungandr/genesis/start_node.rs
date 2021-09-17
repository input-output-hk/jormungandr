use chain_crypto::Ed25519Extended;
use jormungandr_testing_utils::testing::common::{jormungandr::ConfigurationBuilder, startup};

#[test]
pub fn test_genesis_stake_pool_with_account_faucet_starts_successfully() {
    let faucet = startup::create_new_account_address();
    let (_jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();
}

#[ignore]
#[test]
pub fn test_genesis_stake_pool_with_utxo_faucet_starts_successfully() {
    let stake_key = startup::create_new_key_pair::<Ed25519Extended>();
    let faucet = startup::create_new_delegation_address_for(&stake_key.identifier());
    let (_jormungandr, _) =
        startup::start_stake_pool(&[faucet], &[], &mut ConfigurationBuilder::new()).unwrap();
}
