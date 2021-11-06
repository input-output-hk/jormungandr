use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_crypto::{bech32::Bech32, Ed25519Extended, SecretKey};
use jormungandr_lib::interfaces::{
    BlockContentMaxSize, ConfigParam, ConfigParams, ConsensusLeaderId,
};
use jormungandr_testing_utils::testing::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    node::time::{get_current_date, wait_for_epoch},
};
use std::io::Write;

#[test]
fn basic_change_config_test() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let private_key1 = jcli.key().generate_default();
    let private_key2 = jcli.key().generate_default();
    assert_ne!(private_key1, private_key2);

    let sk_file_path1 = temp_dir.join("leader1.sk");
    {
        let mut sk_file1 = std::fs::File::create(&sk_file_path1).unwrap();
        sk_file1.write_all(private_key1.as_bytes()).unwrap();
    }
    let sk_file_path2 = temp_dir.join("leader2.sk");
    {
        let mut sk_file2 = std::fs::File::create(&sk_file_path2).unwrap();
        sk_file2.write_all(private_key2.as_bytes()).unwrap();
    }

    let private_key1 =
        <SecretKey<Ed25519Extended>>::try_from_bech32_str(private_key1.as_str()).unwrap();
    let public_key1 = private_key1.to_public();
    let leader_id1 = ConsensusLeaderId::from(public_key1);
    let private_key2 =
        <SecretKey<Ed25519Extended>>::try_from_bech32_str(private_key2.as_str()).unwrap();
    let public_key2 = private_key2.to_public();
    let leader_id2 = ConsensusLeaderId::from(public_key2);

    let config = ConfigurationBuilder::new()
        .with_consensus_leaders_ids(vec![leader_id1, leader_id2])
        .build(&temp_dir);

    let new_block_context_max_size = 1000;
    let change_params = ConfigParams(vec![ConfigParam::BlockContentMaxSize(
        BlockContentMaxSize::from(new_block_context_max_size),
    )]);
    let change_param_path = temp_dir.child("change_param_file.yaml");
    {
        let content = serde_yaml::to_string(&change_params).unwrap();
        change_param_path.write_str(&content).unwrap();
    }

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let old_settings = jcli.rest().v0().settings(jormungandr.rest_uri());

    let current_epoch = get_current_date(&mut jormungandr.rest()).epoch();

    let fragment = jcli
        .votes()
        .update_proposal(change_param_path, sk_file_path1.clone());
    let check = jcli.fragment_sender(&jormungandr).send(fragment.as_str());
    check.assert_in_block();
    let proposal_id = check.fragment_id();

    let fragment = jcli
        .votes()
        .update_vote(proposal_id.to_string(), sk_file_path1);
    jcli.fragment_sender(&jormungandr).send(fragment.as_str());
    let fragment = jcli
        .votes()
        .update_vote(proposal_id.to_string(), sk_file_path2);
    jcli.fragment_sender(&jormungandr).send(fragment.as_str());

    wait_for_epoch(current_epoch + 2, jormungandr.rest());

    let new_settings = jcli.rest().v0().settings(jormungandr.rest_uri());

    assert_ne!(old_settings, new_settings);
    assert_eq!(
        new_settings.block_content_max_size,
        new_block_context_max_size
    )
}
