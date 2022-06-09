use assert_fs::{prelude::*, TempDir};
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use chain_impl_mockchain::{
    certificate::PoolId, testing::builders::cert_builder::build_stake_pool_retirement_cert,
};
use jormungandr_automation::{jcli::JCli, testing::keys::create_new_key_pair};
use jormungandr_lib::interfaces::Certificate;
use std::str::FromStr;

#[test]
pub fn jcli_creates_correct_retirement_certificate() {
    let jcli: JCli = Default::default();

    let owner = create_new_key_pair::<Ed25519>();
    let kes = create_new_key_pair::<SumEd25519_12>();
    let vrf = create_new_key_pair::<RistrettoGroup2HashDh>();

    let certificate = jcli.certificate().new_stake_pool_registration(
        &kes.identifier().to_bech32_str(),
        &vrf.identifier().to_bech32_str(),
        0,
        1,
        &owner.identifier().to_bech32_str(),
        None,
    );

    let temp_dir = TempDir::new().unwrap();

    let input_file = temp_dir.child("certificate");
    input_file.write_str(&certificate).unwrap();
    let stake_pool_id = jcli.certificate().stake_pool_id(input_file.path()).unwrap();

    let expected_certificate = jcli.certificate().new_stake_pool_retirement(&stake_pool_id);
    let actual_certificate = assert_new_stake_pool_retirement(&stake_pool_id);
    let retirement_cert_file = temp_dir.child("retirement_certificate");
    retirement_cert_file.write_str(&actual_certificate).unwrap();
    let stake_pool_id_from_retirement = jcli
        .certificate()
        .stake_pool_id(retirement_cert_file.path())
        .unwrap();
    assert_eq!(expected_certificate, actual_certificate);
    assert_eq!(stake_pool_id, stake_pool_id_from_retirement);
}

pub fn assert_new_stake_pool_retirement(stake_pool_id: &str) -> String {
    let pool_id = PoolId::from_str(stake_pool_id).unwrap();
    let start_validity = 0u64;
    let certificate = build_stake_pool_retirement_cert(pool_id, start_validity);
    Certificate::from(certificate).to_bech32m().unwrap()
}
