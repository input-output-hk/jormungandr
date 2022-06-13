use assert_fs::{prelude::*, TempDir};
use chain_crypto::{Ed25519, RistrettoGroup2HashDh, SumEd25519_12};
use jormungandr_automation::{jcli::JCli, testing::keys::create_new_key_pair};

#[test]
pub fn test_create_and_sign_new_stake_delegation() {
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
    let certificate = jcli
        .certificate()
        .new_stake_delegation(&stake_pool_id, &owner.identifier().to_bech32_str());

    assert_ne!(certificate, "", "delegation cert is empty");

    let signed_cert = temp_dir.child("signed_cert");
    let owner_private_key_file = temp_dir.child("owner.private");
    owner_private_key_file
        .write_str(&owner.signing_key().to_bech32_str())
        .unwrap();

    jcli.certificate().sign(
        owner_private_key_file.path(),
        input_file.path(),
        signed_cert.path(),
    );

    signed_cert.assert(jortestkit::prelude::file_exists_and_not_empty());
}

#[test]
pub fn test_create_vote_plan_certificate() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let owner = create_new_key_pair::<Ed25519>();
    let owner_private_key_file = temp_dir.child("owner.private");
    owner_private_key_file
        .write_str(&owner.signing_key().to_bech32_str())
        .unwrap();

    let vote_plan_config = r#"
payload_type: public
vote_start:
  epoch: 0
  slot_id: 200
vote_end:
  epoch: 0
  slot_id: 300
committee_end:
  epoch: 0
  slot_id: 400
proposals:
  - external_id: f4fdab54e2d516ce1cabe8ae8cfe77e99eeb530f7033cdf20e2392e012373a7b
    options: 3
    action:
      treasury:
        transfer_to_rewards:
          value: 100
voting_token: "00000000000000000000000000000000000000000000000000000000.00000000"
    "#;

    let vote_plan_config_path = temp_dir.child("vote_plan.yaml");
    std::fs::write(vote_plan_config_path.path(), vote_plan_config).unwrap();

    let certificate = jcli
        .certificate()
        .new_vote_plan(vote_plan_config_path.path());

    assert_ne!(certificate, "", "vote plan cert is empty");
}
