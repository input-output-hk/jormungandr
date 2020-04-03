use crate::common::{
    file_utils, jcli_wrapper::certificate::wrapper::JCLICertificateWrapper,
    startup::create_new_key_pair,
};
use chain_crypto::{Curve25519_2HashDH, Ed25519, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::PoolId, testing::builders::cert_builder::build_stake_pool_retirement_cert,
};
use std::str::FromStr;

use jormungandr_lib::interfaces::Certificate;

#[test]
pub fn jcli_creates_correct_retirement_certificate() {
    let owner = create_new_key_pair::<Ed25519>();
    let kes = create_new_key_pair::<SumEd25519_12>();
    let vrf = create_new_key_pair::<Curve25519_2HashDH>();

    let certificate_wrapper = JCLICertificateWrapper::new();
    let certificate = certificate_wrapper.assert_new_stake_pool_registration(
        &kes.identifier().to_bech32_str(),
        &vrf.identifier().to_bech32_str(),
        0,
        1,
        &owner.identifier().to_bech32_str(),
        None,
    );

    let input_file = file_utils::create_file_in_temp("certificate", &certificate);
    let stake_pool_id = certificate_wrapper.assert_get_stake_pool_id(&input_file);

    let expected_certificate = certificate_wrapper.assert_new_stake_pool_retirement(&stake_pool_id);
    let actual_certificate = assert_new_stake_pool_retirement(&stake_pool_id);

    assert_eq!(expected_certificate, actual_certificate);
}

pub fn assert_new_stake_pool_retirement(stake_pool_id: &str) -> String {
    let pool_id = PoolId::from_str(&stake_pool_id).unwrap();
    let start_validity = 0u64;
    let certificate = build_stake_pool_retirement_cert(pool_id, start_validity);
    format!("{}", Certificate::from(certificate).to_bech32().unwrap())
}
