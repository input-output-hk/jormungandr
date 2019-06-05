#![cfg(feature = "integration-test")]

use common::configuration::{genesis_model::Fund, secret_model::SecretModel};
use common::file_utils;
use common::jcli_wrapper::certificate::wrapper::JCLICertificateWrapper;
use common::startup;

use common::jcli_wrapper;
use common::jcli_wrapper::Discrimination;

#[test]
pub fn test_genesis_stake_pool_starts_successfully() {
    //faucet
    let faucet_sk = jcli_wrapper::assert_key_generate("Ed25519Extended");
    let faucet_pk = jcli_wrapper::assert_key_to_public_default(&faucet_sk);
    let faucet_address = jcli_wrapper::assert_address_account(&faucet_pk, Discrimination::Test);

    // leader
    let leader_sk = jcli_wrapper::assert_key_generate("Ed25519");
    let leader_pk = jcli_wrapper::assert_key_to_public_default(&leader_sk);

    // stake pool
    let pool_vrf_sk = jcli_wrapper::assert_key_generate("Curve25519_2HashDH");
    let pool_vrf_pk = jcli_wrapper::assert_key_to_public_default(&pool_vrf_sk);

    let pool_kes_sk = jcli_wrapper::assert_key_generate("SumEd25519_12");
    let pool_kes_pk = jcli_wrapper::assert_key_to_public_default(&pool_kes_sk);

    // note we use the faucet as the owner to this pool
    let stake_key = faucet_sk;
    let stake_key_pub = faucet_pk;

    let stake_key_file = file_utils::create_file_in_temp("stake_key.sk", &stake_key);

    let jcli_certificate = JCLICertificateWrapper::new();

    let stake_pool_cert = jcli_certificate.assert_new_stake_pool_registration(
        &pool_kes_pk,
        "1010101010",
        &pool_vrf_pk,
    );
    let stake_pool_cert_file = file_utils::create_file_in_temp("stake_pool.cert", &stake_pool_cert);

    let stake_pool_signcert_file = file_utils::get_path_in_temp("stake_pool.signcert");
    jcli_certificate.assert_sign(
        &stake_key_file,
        &stake_pool_cert_file,
        &stake_pool_signcert_file,
    );

    let stake_pool_id_file = file_utils::get_path_in_temp("stake_pool.id");
    let stake_pool_id =
        jcli_certificate.assert_get_stake_pool_id(&stake_pool_signcert_file, &stake_pool_id_file);
    let stake_pool_signcert = file_utils::read_file(&stake_pool_signcert_file);

    let stake_delegation_cert =
        jcli_certificate.assert_new_stake_delegation(&stake_pool_id, &stake_key_pub);

    let stake_delegation_cert_file =
        file_utils::create_file_in_temp("stake_delegation.cert", &stake_delegation_cert);
    let stake_delegation_signcert_file = file_utils::get_path_in_temp("stake_delegation.signcert");

    jcli_certificate.assert_sign(
        &stake_key_file,
        &stake_delegation_cert_file,
        &stake_delegation_signcert_file,
    );
    let stake_delegation_signcert = file_utils::read_file(&stake_delegation_signcert_file);

    let mut config = startup::ConfigurationBuilder::new()
        .with_block0_consensus("genesis")
        .with_bft_slots_ratio("0".to_owned())
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_consensus_leaders_ids(vec![leader_pk.clone()])
        .with_kes_update_speed(43200)
        .with_allow_account_creation(true)
        .with_initial_certs(vec![
            stake_pool_signcert.clone(),
            stake_delegation_signcert.clone(),
        ])
        .with_funds(vec![Fund {
            address: faucet_address.clone(),
            value: 100,
        }])
        .build();

    let secret = SecretModel::new_genesis(&pool_kes_sk, &pool_vrf_sk, &stake_pool_id);
    let secret_file = SecretModel::serialize(&secret);
    config.secret_model = secret;
    config.secret_model_path = secret_file;
    let _jormungandr = startup::start_jormungandr_node(&mut config);
}
