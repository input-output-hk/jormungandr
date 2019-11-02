use crate::common::{
    configuration::{genesis_model::Fund, secret_model::SecretModel},
    file_utils,
    jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

#[test]
pub fn test_genesis_stake_pool_with_account_faucet_starts_successfully() {
    //faucet
    let faucet = startup::create_new_account_address();

    // leader
    let leader = startup::create_new_key_pair("Ed25519");

    // stake pool
    let pool_vrf = startup::create_new_key_pair("Curve25519_2HashDH");
    let pool_kes = startup::create_new_key_pair("SumEd25519_12");

    // note we use the faucet as the owner to this pool
    let stake_key = faucet.private_key;
    let stake_key_pub = faucet.public_key;

    let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key);

    let jcli_certificate = JCLICertificateWrapper::new();

    let stake_pool_signcert_file = jcli_certificate.assert_new_signed_stake_pool_cert(
        &pool_kes.public_key,
        "1010101010",
        &pool_vrf.public_key,
        &stake_key_file,
        0,
        1,
        &stake_key_pub,
    );
    let stake_pool_signcert = file_utils::read_file(&stake_pool_signcert_file);

    let stake_pool_id = jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file);

    let stake_delegation_signcert = jcli_certificate.assert_new_signed_stake_pool_delegation(
        &stake_pool_id,
        &stake_key_pub,
        &stake_key_file,
    );

    let mut config = ConfigurationBuilder::new()
        .with_block0_consensus("genesis_praos")
        .with_bft_slots_ratio("0".to_owned())
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(vec![leader.public_key.clone()])
        .with_kes_update_speed(43200)
        .with_initial_certs(vec![
            stake_pool_signcert.clone(),
            stake_delegation_signcert.clone(),
        ])
        .with_funds(vec![Fund {
            address: faucet.address.clone(),
            value: 100.into(),
        }])
        .build();

    let secret =
        SecretModel::new_genesis(&pool_kes.private_key, &pool_vrf.private_key, &stake_pool_id);
    let secret_file = SecretModel::serialize(&secret);
    config.secret_model = secret;
    config.secret_model_path = secret_file;
    let _jormungandr = Starter::new().config(config).start().unwrap();
}

#[test]
pub fn test_genesis_stake_pool_with_utxo_faucet_starts_successfully() {
    // stake key
    let stake_key = startup::create_new_key_pair("Ed25519Extended");
    //faucet
    let faucet = startup::create_new_delegation_address_for(&stake_key.public_key);
    // leader
    let leader = startup::create_new_key_pair("Ed25519");

    // stake pool
    let pool_vrf = startup::create_new_key_pair("Curve25519_2HashDH");
    let pool_kes = startup::create_new_key_pair("SumEd25519_12");

    // note we use the faucet as the owner to this pool
    let owner_key = faucet.private_key;
    let owner_pubkey = faucet.public_key;
    let stake_key_pub = &faucet.delegation_key;

    let owner_key_file = file_utils::create_file_in_temp("owner_key.sk", &owner_key);
    let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key.private_key);

    let jcli_certificate = JCLICertificateWrapper::new();

    let stake_pool_signcert_file = jcli_certificate.assert_new_signed_stake_pool_cert(
        &pool_kes.public_key,
        "1010101010",
        &pool_vrf.public_key,
        &owner_key_file,
        0,
        1,
        &owner_pubkey,
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
        .with_bft_slots_ratio("0".to_owned())
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(vec![leader.public_key.clone()])
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

    let secret =
        SecretModel::new_genesis(&pool_kes.private_key, &pool_vrf.private_key, &stake_pool_id);
    let secret_file = SecretModel::serialize(&secret);
    config.secret_model = secret;
    config.secret_model_path = secret_file;

    let _jormungandr = Starter::new().config(config).start().unwrap();
}
