use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_crypto::bech32::Bech32;
use chain_impl_mockchain::value::Value;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    testing::time::{get_current_date, wait_for_epoch},
};
use jormungandr_lib::interfaces::{
    BlockContentMaxSize, BlockDate, ConfigParam, ConfigParams, ConsensusLeaderId,
};
use jortestkit::process::Wait;
use std::time::Duration;

#[test]
fn basic_change_config_test() {
    let temp_dir = TempDir::new().unwrap();

    let jcli: JCli = Default::default();
    let wallet_initial_funds = 1_000_000;

    let mut alice = thor::Wallet::default();
    let alice_sk = temp_dir.child("alice_sk");
    alice.save_to_path(alice_sk.path()).unwrap();

    let mut bob = thor::Wallet::default();
    let bob_sk = temp_dir.child("bob_sk");
    bob.save_to_path(bob_sk.path()).unwrap();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            alice.to_initial_fund(wallet_initial_funds),
            bob.to_initial_fund(wallet_initial_funds),
        ])
        .with_consensus_leaders_ids(vec![
            ConsensusLeaderId::from(bob.public_key()),
            ConsensusLeaderId::from(alice.public_key()),
        ])
        .build(&temp_dir);

    let new_block_context_max_size = 1000;
    let change_params = ConfigParams::new(vec![ConfigParam::BlockContentMaxSize(
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

    let wait = Wait::new(Duration::from_secs(5), 10);

    let update_proposal_cert = jcli
        .certificate()
        .new_update_proposal(&alice.public_key().to_bech32_str(), change_param_path);
    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&update_proposal_cert)
        .set_expiry_date(BlockDate::new(3, 0))
        .finalize()
        .seal_with_witness_data(alice.witness_data())
        .add_auth(alice_sk.path())
        .to_message();
    alice.confirm_transaction();
    let check = jcli.fragment_sender(&jormungandr).send(tx.as_str());
    check.assert_in_block_with_wait(&wait);
    let proposal_id = check.fragment_id();

    let update_vote_cert = jcli.certificate().new_update_vote(
        &proposal_id.to_string(),
        &alice.public_key().to_bech32_str(),
    );
    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&update_vote_cert)
        .set_expiry_date(BlockDate::new(3, 0))
        .finalize()
        .seal_with_witness_data(alice.witness_data())
        .add_auth(alice_sk.path())
        .to_message();
    alice.confirm_transaction();
    jcli.fragment_sender(&jormungandr)
        .send(tx.as_str())
        .assert_in_block_with_wait(&wait);

    let update_vote_cert = jcli
        .certificate()
        .new_update_vote(&proposal_id.to_string(), &bob.public_key().to_bech32_str());
    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&bob.address().to_string(), &Value::zero().into())
        .add_certificate(&update_vote_cert)
        .set_expiry_date(BlockDate::new(3, 0))
        .finalize()
        .seal_with_witness_data(bob.witness_data())
        .add_auth(bob_sk.path())
        .to_message();
    bob.confirm_transaction();
    jcli.fragment_sender(&jormungandr)
        .send(tx.as_str())
        .assert_in_block_with_wait(&wait);

    wait_for_epoch(current_epoch + 2, jormungandr.rest());

    let new_settings = jcli.rest().v0().settings(jormungandr.rest_uri());

    assert_ne!(old_settings, new_settings);
    assert_eq!(
        new_settings.block_content_max_size,
        new_block_context_max_size
    )
}
