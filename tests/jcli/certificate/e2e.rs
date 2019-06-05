#![cfg(feature = "integration-test")]

use common::file_assert;
use common::file_utils;
use common::jcli_wrapper;
use common::jcli_wrapper::certificate::wrapper::JCLICertificateWrapper;

#[test]
pub fn test_create_and_sign_new_stake_delegation() {
    let owner_private_key = jcli_wrapper::assert_key_generate_default();
    let owner_public_key = jcli_wrapper::assert_key_to_public_default(&owner_private_key);

    let kes_private_key = jcli_wrapper::assert_key_generate("SumEd25519_12");
    let kes_public_key = jcli_wrapper::assert_key_to_public_default(&kes_private_key);

    let vrf_private_key = jcli_wrapper::assert_key_generate("Curve25519_2HashDH");
    let vrf_public_key = jcli_wrapper::assert_key_to_public_default(&vrf_private_key);

    let serial_id = "13919597664319319060838570079442950054";

    let certificate_wrapper = JCLICertificateWrapper::new();
    let certificate = certificate_wrapper.assert_new_stake_pool_registration(
        &kes_public_key,
        &serial_id,
        &vrf_public_key,
    );

    let input_file = file_utils::create_file_in_temp("certificate", &certificate);
    let output_file = file_utils::get_path_in_temp("pool_id");

    let stake_pool_id = certificate_wrapper.assert_get_stake_pool_id(&input_file, &output_file);
    let certificate =
        certificate_wrapper.assert_new_stake_delegation(&stake_pool_id, &owner_public_key);

    assert_ne!(certificate, "", "delegation cert is empty");

    let signed_cert = file_utils::get_path_in_temp("signed_cert");
    let owner_private_key_file =
        file_utils::create_file_in_temp("owner.private", &owner_private_key);

    certificate_wrapper.assert_sign(&owner_private_key_file, &input_file, &signed_cert);

    file_assert::assert_file_exists_and_not_empty(&signed_cert);
}
