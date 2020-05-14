use crate::common::{
    configuration::SecretModelFactory,
    file_utils,
    jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, Ed25519Extended, SumEd25519_12};
use chain_impl_mockchain::chaintypes::ConsensusVersion;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, InitialUTxO, KESUpdateSpeed};

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
