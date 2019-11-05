use crate::common::{
    file_assert, file_utils, jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    startup::create_new_key_pair,
};

use chain_crypto::{Curve25519_2HashDH, Ed25519, SumEd25519_12};

#[test]
pub fn test_create_and_sign_new_stake_delegation() {
    let owner = create_new_key_pair::<Ed25519>();
    let kes = create_new_key_pair::<SumEd25519_12>();
    let vrf = create_new_key_pair::<Curve25519_2HashDH>();

    let serial_id = "13919597664319319060838570079442950054";

    let certificate_wrapper = JCLICertificateWrapper::new();
    let certificate = certificate_wrapper.assert_new_stake_pool_registration(
        &kes.identifier().to_bech32_str(),
        &serial_id,
        &vrf.identifier().to_bech32_str(),
        0,
        1,
        &owner.identifier().to_bech32_str(),
    );

    let input_file = file_utils::create_file_in_temp("certificate", &certificate);
    let stake_pool_id = certificate_wrapper.assert_get_stake_pool_id(&input_file);
    let certificate = certificate_wrapper
        .assert_new_stake_delegation(&stake_pool_id, &owner.identifier().to_bech32_str());

    assert_ne!(certificate, "", "delegation cert is empty");

    let signed_cert = file_utils::get_path_in_temp("signed_cert");
    let owner_private_key_file =
        file_utils::create_file_in_temp("owner.private", &owner.signing_key().to_bech32_str());

    certificate_wrapper.assert_sign(&owner_private_key_file, &input_file, &signed_cert);

    file_assert::assert_file_exists_and_not_empty(&signed_cert);
}
