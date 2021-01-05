use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::NamedTempFile;
use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use bech32::FromBase32;
use chain_addr::Discrimination;
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::VoteAction, chaintypes::ConsensusType, milli::Milli, value::Value, vote::Choice,
};
use chain_vote::MemberPublicKey;
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, FeesGoTo, KESUpdateSpeed};
use jormungandr_testing_utils::testing::node::time;
use jormungandr_testing_utils::testing::{VotePlanBuilder, VotePlanExtension};
use jormungandr_testing_utils::wallet::Wallet;
use jortestkit::prelude::read_file;
use rand::rngs::OsRng;
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
    let communication_pk = jcli
        .votes()
        .committee()
        .communication_key()
        .to_public(communication_sk);
    let crs = jcli.votes().crs().generate();
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
    let encrypting_vote_key = jcli.votes().encrypting_vote_key(member_pk.clone());

    let member_sk_file = NamedTempFile::new("member.sk").unwrap();
    member_sk_file.write_str(&member_sk).unwrap();

    let (_, member_pk_bech32) = bech32::decode(&member_pk).unwrap();
    let member_pk_bytes = Vec::<u8>::from_base32(&member_pk_bech32).unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .private()
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .member_public_key(MemberPublicKey::from_bytes(&member_pk_bytes).unwrap())
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

    let encrypted_vote_tally = NamedTempFile::new("encrypted-vote-tally.certificate").unwrap();

    jcli.certificate()
        .new_encrypted_vote_tally(vote_plan_id, encrypted_vote_tally.path());

    let vote_tally_cert = read_file(encrypted_vote_tally.path());

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
    let vote_tally_file = NamedTempFile::new("vote_tally_proposal_0.yaml").unwrap();

    let encrypted_tally = match vote_tally
        .get(0)
        .unwrap()
        .proposals
        .get(0)
        .unwrap()
        .tally
        .as_ref()
        .unwrap()
    {
        jormungandr_lib::interfaces::Tally::Private { state } => match state {
            jormungandr_lib::interfaces::PrivateTallyState::Encrypted {
                encrypted_tally,
                total_stake: _,
            } => serde_json::to_string(&encrypted_tally)
                .unwrap()
                .replace("\"", ""),
            _ => panic!("tally state should be encrypted"),
        },
        _ => panic!("voting should be private"),
    };

    vote_tally_file.write_str(&encrypted_tally).unwrap();

    let decryption_share = jcli
        .votes()
        .tally()
        .generate_decryption_share(member_sk_file.path(), vote_tally_file.path());

    let decryption_share_file = NamedTempFile::new("decryption_share").unwrap();
    decryption_share_file.write_str(&decryption_share).unwrap();

    let generated_share = jcli.votes().tally().decrypt_with_shares(
        vote_tally_file.path(),
        3,
        decryption_share_file.path(),
        1,
        1,
    );

    println!("{}", generated_share);
}
