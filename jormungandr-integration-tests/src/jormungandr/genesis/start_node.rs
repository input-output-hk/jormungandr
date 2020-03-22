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

#[test]
pub fn test_genesis_stake_pool_with_utxo_faucet_starts_successfully() {
    // stake key
    let stake_key = startup::create_new_key_pair::<Ed25519Extended>();
    //faucet
    let faucet = startup::create_new_delegation_address_for(&stake_key.identifier());
    // leader
    let leader = startup::create_new_key_pair::<Ed25519>();

    // stake pool
    let pool_vrf = startup::create_new_key_pair::<Curve25519_2HashDH>();
    let pool_kes = startup::create_new_key_pair::<SumEd25519_12>();

    // note we use the faucet as the owner to this pool
    let owner_key = faucet.signing_key_as_str();
    let owner_pubkey = faucet.identifier();
    let stake_key_pub = &faucet.delegation_key();

    let owner_key_file = file_utils::create_file_in_temp("owner_key.sk", &owner_key);
    let stake_key_file =
        file_utils::create_file_in_temp("stake_key.sk", &stake_key.signing_key().to_bech32_str());

    let jcli_certificate = JCLICertificateWrapper::new();

    let stake_pool_signcert_file = jcli_certificate.assert_new_signed_stake_pool_cert(
        &pool_kes.identifier().to_bech32_str(),
        &pool_vrf.identifier().to_bech32_str(),
        &owner_key_file,
        0,
        1,
        &owner_pubkey.to_bech32_str(),
        None,
    );
    let stake_pool_signcert = file_utils::read_file(&stake_pool_signcert_file);

    let stake_pool_id = jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file);

    // WRONG
    let stake_delegation_signcert = jcli_certificate.assert_new_signed_stake_pool_delegation(
        &stake_pool_id,
        &stake_key_pub.to_bech32_str(),
        &stake_key_file,
    );

    let mut config = ConfigurationBuilder::new()
        .with_block0_consensus(ConsensusVersion::GenesisPraos)
        .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
        .with_consensus_leaders_ids(vec![leader.identifier().into()])
        .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap())
        .with_funds(vec![InitialUTxO {
            address: faucet.address(),
            value: 100.into(),
        }])
        .with_initial_certs(vec![
            stake_pool_signcert.clone(),
            stake_delegation_signcert.clone(),
        ])
        .build();

    let secret = SecretModelFactory::genesis(
        pool_kes.signing_key(),
        pool_vrf.signing_key(),
        &stake_pool_id,
    );
    let secret_file = SecretModelFactory::serialize(&secret);
    config.secret_models = vec![secret];
    config.secret_model_paths = vec![secret_file];

    let _jormungandr = Starter::new().config(config).start().unwrap();
}
