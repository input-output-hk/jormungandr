use assert_fs::NamedTempFile;
use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use bech32::FromBase32;
use chain_addr::Discrimination;
use chain_core::property::BlockDate as _;
use chain_impl_mockchain::header::BlockDate;
use chain_impl_mockchain::{
    certificate::VoteAction, chaintypes::ConsensusType,
    ledger::governance::TreasuryGovernanceAction, value::Value, vote::Choice,
};
use chain_vote::MemberPublicKey;
use jormungandr_lib::interfaces::{BlockDate as BlockDateDto, KesUpdateSpeed};
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
};
use jormungandr_testing_utils::testing::{VotePlanBuilder, VotePlanExtension};
use jormungandr_testing_utils::wallet::Wallet;
use jortestkit::prelude::read_file;
use rand::rngs::OsRng;

#[test]
pub fn jcli_e2e_flow_private_vote() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let rewards_increase = 10;
    let yes_choice = Choice::new(1);
    let no_choice = Choice::new(2);

    let wallet_initial_funds = 1_000_000;

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let bob = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let clarice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let communication_sk = jcli.votes().committee().communication_key().generate();
    let communication_pk = jcli
        .votes()
        .committee()
        .communication_key()
        .to_public(communication_sk);
    let crs = "Committee member crs";
    let member_sk =
        jcli.votes()
            .committee()
            .member_key()
            .generate(communication_pk, crs, 0, 1, None);
    let member_pk = jcli
        .votes()
        .committee()
        .member_key()
        .to_public(member_sk.clone());
    let election_public_key = jcli.votes().election_public_key(member_pk.clone());

    let member_sk_file = NamedTempFile::new("member.sk").unwrap();
    member_sk_file.write_str(&member_sk).unwrap();

    let (_, member_pk_bech32) = bech32::decode(&member_pk).unwrap();
    let member_pk_bytes = Vec::<u8>::from_base32(&member_pk_bech32).unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .private()
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .member_public_key(MemberPublicKey::from_bytes(&member_pk_bytes).unwrap())
        .options_size(3)
        .build();

    let vote_plan_json = temp_dir.child("vote_plan.json");
    vote_plan_json.write_str(&vote_plan.as_json_str()).unwrap();

    let vote_plan_cert = jcli.certificate().new_vote_plan(vote_plan_json.path());

    let config = ConfigurationBuilder::new()
        .with_explorer()
        .with_funds(vec![
            alice.to_initial_fund(wallet_initial_funds),
            bob.to_initial_fund(wallet_initial_funds),
            clarice.to_initial_fund(wallet_initial_funds),
        ])
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap())
        .with_treasury(1000.into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[&alice])
        .with_slot_duration(4)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start().unwrap();

    let alice_sk = temp_dir.child("alice_sk");
    alice.save_to_path(alice_sk.path()).unwrap();

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_plan_cert)
        .set_expiry_date(BlockDateDto::new(1, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    let rewards_before = jormungandr.explorer().last_block().unwrap().rewards();

    time::wait_for_epoch(1, jormungandr.rest());

    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert);
    let yes_vote_cast = jcli.certificate().new_private_vote_cast(
        vote_plan_id.clone(),
        0,
        yes_choice,
        3,
        election_public_key.clone(),
    );

    let no_vote_cast = jcli.certificate().new_private_vote_cast(
        vote_plan_id.clone(),
        0,
        no_choice,
        3,
        election_public_key,
    );

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&yes_vote_cast)
        .set_expiry_date(BlockDateDto::new(2, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&bob.address().to_string(), &Value::zero().into())
        .add_certificate(&yes_vote_cast)
        .set_expiry_date(BlockDateDto::new(2, 0))
        .finalize()
        .seal_with_witness_for_address(&bob)
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&clarice.address().to_string(), &Value::zero().into())
        .add_certificate(&no_vote_cast)
        .set_expiry_date(BlockDateDto::new(2, 0))
        .finalize()
        .seal_with_witness_for_address(&clarice)
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(2, jormungandr.rest());

    let encrypted_vote_tally = temp_dir.child("encrypted-vote-tally.certificate");

    jcli.certificate()
        .new_encrypted_vote_tally(vote_plan_id.clone(), encrypted_vote_tally.path());

    let encrypted_vote_tally_cert = read_file(encrypted_vote_tally.path());

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&encrypted_vote_tally_cert)
        .set_expiry_date(BlockDateDto::new(3, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    let active_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    let active_plans_file = temp_dir.child("active_plans.json");
    active_plans_file
        .write_str(&serde_json::to_string(&active_plans).unwrap())
        .unwrap();

    let decryption_shares = jcli.votes().tally().decryption_shares(
        active_plans_file.path(),
        &vote_plan_id,
        member_sk_file.path(),
    );

    let decryption_share_file = temp_dir.child("decryption_share.json");
    decryption_share_file.write_str(&decryption_shares).unwrap();

    let merged_shares = jcli
        .votes()
        .tally()
        .merge_shares(vec![decryption_share_file.path()]);
    let merged_shares_file = temp_dir.child("shares.json");
    merged_shares_file.write_str(&merged_shares).unwrap();

    let result = jcli.votes().tally().decrypt_results(
        active_plans_file.path(),
        &vote_plan_id,
        merged_shares_file.path(),
        1,
    );

    let result_file = temp_dir.child("result.json");
    result_file.write_str(&result).unwrap();

    let vote_tally_cert = jcli.certificate().new_private_vote_tally(
        result_file.path(),
        vote_plan_id,
        merged_shares_file.path(),
    );

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_tally_cert)
        .set_expiry_date(BlockDateDto::new(3, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(3, jormungandr.rest());

    let rewards_after = jormungandr.explorer().last_block().unwrap().rewards();
    // We want to make sure that our small rewards increase is reflexed in current rewards amount
    assert!(
        rewards_after == rewards_before + rewards_increase,
        "Vote was unsuccessful"
    );
}

#[test]
pub fn jcli_private_vote_invalid_proof() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let wallet_initial_funds = 1_000_000;

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let communication_sk = jcli.votes().committee().communication_key().generate();
    let communication_pk = jcli
        .votes()
        .committee()
        .communication_key()
        .to_public(communication_sk);
    let crs = "Committee member crs";

    let invald_crs = "Invalid Committee member crs";

    let member_sk =
        jcli.votes()
            .committee()
            .member_key()
            .generate(communication_pk.clone(), crs, 0, 1, None);
    let member_pk = jcli
        .votes()
        .committee()
        .member_key()
        .to_public(member_sk.clone());

    let member_sk_file = NamedTempFile::new("member.sk").unwrap();
    member_sk_file.write_str(&member_sk).unwrap();

    let invalid_member_sk =
        jcli.votes()
            .committee()
            .member_key()
            .generate(communication_pk, invald_crs, 0, 1, None);

    let invalid_member_sk_file = NamedTempFile::new("member.sk").unwrap();
    invalid_member_sk_file
        .write_str(&invalid_member_sk)
        .unwrap();

    let (_, member_pk_bech32) = bech32::decode(&member_pk).unwrap();
    let member_pk_bytes = Vec::<u8>::from_base32(&member_pk_bech32).unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::OffChain)
        .private()
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 1))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .member_public_key(MemberPublicKey::from_bytes(&member_pk_bytes).unwrap())
        .options_size(3)
        .build();

    let vote_plan_json = temp_dir.child("vote_plan.json");
    vote_plan_json.write_str(&vote_plan.as_json_str()).unwrap();

    let vote_plan_cert = jcli.certificate().new_vote_plan(vote_plan_json.path());

    let config = ConfigurationBuilder::new()
        .with_explorer()
        .with_funds(vec![alice.to_initial_fund(wallet_initial_funds)])
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap())
        .with_treasury(1000.into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[&alice])
        .with_slot_duration(4)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start().unwrap();

    let alice_sk = temp_dir.child("alice_sk");
    alice.save_to_path(alice_sk.path()).unwrap();

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_plan_cert)
        .set_expiry_date(BlockDateDto::new(1, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    time::wait_for_date(
        BlockDate::from_epoch_slot_id(1, 1).into(),
        jormungandr.rest(),
    );

    let encrypted_vote_tally = temp_dir.child("encrypted-vote-tally.certificate");
    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert);

    jcli.certificate()
        .new_encrypted_vote_tally(vote_plan_id.clone(), encrypted_vote_tally.path());

    let encrypted_vote_tally_cert = read_file(encrypted_vote_tally.path());

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&encrypted_vote_tally_cert)
        .set_expiry_date(BlockDateDto::new(2, 0))
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    let active_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    let active_plans_file = temp_dir.child("active_plans.json");
    active_plans_file
        .write_str(&serde_json::to_string(&active_plans).unwrap())
        .unwrap();

    let decryption_shares = jcli.votes().tally().decryption_shares(
        active_plans_file.path(),
        &vote_plan_id,
        invalid_member_sk_file.path(),
    );

    let decryption_share_file = temp_dir.child("decryption_share.json");
    decryption_share_file.write_str(&decryption_shares).unwrap();

    let merged_shares = jcli
        .votes()
        .tally()
        .merge_shares(vec![decryption_share_file.path()]);
    let merged_shares_file = temp_dir.child("shares.json");
    merged_shares_file.write_str(&merged_shares).unwrap();

    jcli.votes().tally().decrypt_results_expect_fail(
        active_plans_file.path(),
        &vote_plan_id,
        merged_shares_file.path(),
        1,
        "Incorrect decryption shares",
    );
}
