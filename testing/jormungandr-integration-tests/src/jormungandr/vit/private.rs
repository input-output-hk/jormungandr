use crate::common::startup::start_stake_pool;
use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_addr::Discrimination;
use chain_impl_mockchain::{
    certificate::{VoteAction, VotePlan},
    chaintypes::ConsensusType,
    milli::Milli,
    testing::VoteTestGen,
    value::Value,
    vote::{Choice, CommitteeId, PayloadType},
};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{
        ActiveSlotCoefficient, CommitteeIdDef, FeesGoTo, KESUpdateSpeed, Tally, VotePlanStatus,
    },
};
use jormungandr_testing_utils::testing::VotePlanExtension;
use jormungandr_testing_utils::{
    testing::{
        node::time::{self, wait_for_epoch},
        vote_plan_cert, FragmentSender, FragmentSenderSetup,
    },
    wallet::Wallet,
};
use rand::rngs::OsRng;
use rand_core::{CryptoRng, RngCore};

#[test]
pub fn jcli_e2e_flow_private_vote() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let yes_choice = Choice::new(1);

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let bob = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let clarice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let communication_sk = jcli.votes().committee().communication_key().generate();
    let crs = jcli.votes().crs().generate();
    let member_sk = jcli
        .votes()
        .committee()
        .member_key()
        .generate(communication_sk, crs, 1, 1);
    let member_pk = jcli.votes().committee().member_key().to_public(member_sk);
    let encrypting_vote_key = jcli.votes().encrypting_vote_key(member_pk);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .actionType(VoteAction::OffChain)
        .private()
        .member_public_key(member_pk)
        .build();

    let vote_plan_json = temp_dir.child("vote_plan.json");
    vote_plan_json.write_str(&vote_plan.as_json_str()).unwrap();

    let config = ConfigurationBuilder::new()
        .with_explorer()
        .with_funds(vec![
            alice.to_initial_fund(1_000_000),
            bob.to_initial_fund(1_000_000),
            clarice.to_initial_fund(1_000_000),
        ])
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap())
        .with_fees_go_to(FeesGoTo::Rewards)
        .with_treasury(Value::zero().into())
        .with_total_rewards_supply(Value::zero().into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[&alice])
        .with_slots_per_epoch(60)
        .with_consensus_genesis_praos_active_slot_coeff(
            ActiveSlotCoefficient::new(Milli::from_millis(100)).unwrap(),
        )
        .with_slot_duration(4)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config).start().unwrap();

    let alice_sk = temp_dir.child("alice_sk");
    alice.save_to_path(alice_sk.path()).unwrap();

    let vote_plan_cert = jcli.certificate().new_vote_plan(vote_plan_json.path());

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_plan_cert)
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();
    time::wait_for_epoch(1, jormungandr.explorer());

    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert);
    let vote_cast = jcli.certificate().new_private_vote_cast(
        vote_plan_id.clone(),
        0,
        yes_choice,
        3,
        encrypting_vote_key,
    );

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_cast)
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
        .add_certificate(&vote_cast)
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
        .add_certificate(&vote_cast)
        .finalize()
        .seal_with_witness_for_address(&clarice)
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(2, jormungandr.explorer());

    let vote_tally_cert = jcli.certificate().new_vote_tally(vote_plan_id);

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_tally_cert)
        .finalize()
        .seal_with_witness_for_address(&alice)
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(3, jormungandr.explorer());

    let vote_tally = jormungandr.rest().vote_plan_statuses().unwrap();

    let decryption_share = jcli.votes().tally().generate_decryption_share();
    let generated_share = jcli.votes().tally().decrypt_with_shares();

    assert!(jormungandr
        .rest()
        .vote_plan_statuses()
        .unwrap()
        .first()
        .unwrap()
        .proposals
        .first()
        .unwrap()
        .tally
        .is_some());

    assert_eq!(
        jormungandr
            .rest()
            .vote_plan_statuses()
            .unwrap()
            .first()
            .unwrap()
            .proposals
            .first()
            .unwrap()
            .votes_cast,
        3
    );
}
