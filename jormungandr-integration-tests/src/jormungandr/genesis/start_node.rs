use crate::common::{
    configuration::{genesis_model::Fund, secret_model::SecretModel},
    file_utils,
    jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, Ed25519Extended, SumEd25519_12};

#[test]
pub fn test_genesis_stake_pool_with_account_faucet_starts_successfully() {
    let faucet = startup::create_new_account_address();
    let (_jormungandr, _) =
        startup::start_stake_pool(&[faucet], &mut ConfigurationBuilder::new()).unwrap();
}

#[test]
pub fn test_genesis_stake_pool_with_utxo_faucet_starts_successfully() {
    // stake key
    let stake_key = startup::create_new_key_pair::<Ed25519Extended>();
    //faucet
    let faucet =
        startup::create_new_delegation_address_for(&stake_key.identifier().to_bech32_str());
    // leader
    let leader = startup::create_new_key_pair::<Ed25519>();

    // stake pool
    let pool_vrf = startup::create_new_key_pair::<Curve25519_2HashDH>();
    let pool_kes = startup::create_new_key_pair::<SumEd25519_12>();

    // note we use the faucet as the owner to this pool
    let owner_key = faucet.private_key;
    let owner_pubkey = faucet.public_key;
    let stake_key_pub = &faucet.delegation_key;

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
        &owner_pubkey,
        None,
    );
    let stake_pool_signcert = file_utils::read_file(&stake_pool_signcert_file);

    let stake_pool_id = jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file);

    // WRONG
    let stake_delegation_signcert = jcli_certificate.assert_new_signed_stake_pool_delegation(
        &stake_pool_id,
        &stake_key_pub,
        &stake_key_file,
    );

    let mut config = ConfigurationBuilder::new()
        .with_block0_consensus("genesis_praos")
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(vec![leader.identifier().to_bech32_str()])
        .with_kes_update_speed(43200)
        .with_funds(vec![Fund {
            address: faucet.address.clone(),
            value: 100.into(),
        }])
        .with_initial_certs(vec![
            stake_pool_signcert.clone(),
            stake_delegation_signcert.clone(),
        ])
        .build();

    let secret = SecretModel::new_genesis(
        &pool_kes.signing_key().to_bech32_str(),
        &pool_vrf.signing_key().to_bech32_str(),
        &stake_pool_id,
    );
    let secret_file = SecretModel::serialize(&secret);
    config.secret_models = vec![secret];
    config.secret_model_paths = vec![secret_file];

    let _jormungandr = Starter::new().config(config).start().unwrap();
}
