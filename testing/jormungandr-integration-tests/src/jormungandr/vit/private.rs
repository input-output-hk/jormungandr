use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    NamedTempFile, TempDir,
};
use bech32::FromBase32;
use chain_addr::Discrimination;
use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    certificate::VoteAction, chaintypes::ConsensusType, header::BlockDate,
    ledger::governance::TreasuryGovernanceAction, tokens::minting_policy::MintingPolicy,
    value::Value, vote::Choice,
};
use chain_vote::MemberPublicKey;
use core::time::Duration;
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    testing::{time, time::wait_for_epoch, VotePlanBuilder, VotePlanExtension},
};
use jormungandr_lib::interfaces::{
    BlockDate as BlockDateDto, InitialToken, KesUpdateSpeed, NodeState,
};
use rand::rngs::OsRng;
use thor::{
    vote_plan_cert, CommitteeDataManager, FragmentSender, FragmentSenderSetup, FragmentVerifier,
    Wallet,
};

const INITIAL_FUND_PER_WALLET: u64 = 1_000_000;
const INITIAL_TREASURY: u64 = 1000;
const REWARD_INCREASE: u64 = 10;
const SLOTS_PER_EPOCH: u32 = 10;
const SLOT_DURATION: u8 = 4;

#[test]
pub fn jcli_e2e_flow_private_vote() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();
    let yes_choice = Choice::new(1);
    let no_choice = Choice::new(2);

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let bob = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let clarice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let communication_sk = jcli.votes().committee().communication_key().generate();
    let communication_pk = jcli
        .votes()
        .committee()
        .communication_key()
        .to_public(communication_sk)
        .unwrap();
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
        .to_public(member_sk.clone())
        .unwrap();
    let election_public_key = jcli.votes().election_public_key(member_pk.clone()).unwrap();

    let member_sk_file = NamedTempFile::new("member.sk").unwrap();
    member_sk_file.write_str(&member_sk).unwrap();

    let (_, member_pk_bech32) = bech32::decode(&member_pk).unwrap();
    let member_pk_bytes = Vec::<u8>::from_base32(&member_pk_bech32).unwrap();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(REWARD_INCREASE),
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

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            alice.to_initial_fund(INITIAL_FUND_PER_WALLET),
            bob.to_initial_fund(INITIAL_FUND_PER_WALLET),
            clarice.to_initial_fund(INITIAL_FUND_PER_WALLET),
        ])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![
                alice.to_initial_token(INITIAL_FUND_PER_WALLET),
                bob.to_initial_token(INITIAL_FUND_PER_WALLET),
                clarice.to_initial_token(INITIAL_FUND_PER_WALLET),
            ],
        })
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KesUpdateSpeed::MAXIMUM) //KesUpdateSpeed::new(43200).unwrap()
        .with_treasury(INITIAL_TREASURY.into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[alice.to_committee_id()])
        .with_slot_duration(SLOT_DURATION)
        .with_slots_per_epoch(SLOTS_PER_EPOCH)
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
        .seal_with_witness_data(alice.witness_data())
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    alice.confirm_transaction();

    let rewards_before: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    time::wait_for_epoch(1, jormungandr.rest());

    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert).unwrap();
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
        .seal_with_witness_data(alice.witness_data())
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
        .seal_with_witness_data(bob.witness_data())
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
        .seal_with_witness_data(clarice.witness_data())
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(2, jormungandr.rest());

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

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
        .seal_with_witness_data(alice.witness_data())
        .add_auth(alice_sk.path())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(3, jormungandr.rest());

    let rewards_after: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    // We want to make sure that our small rewards increase is reflected in current rewards amount
    assert!(
        rewards_after == rewards_before + REWARD_INCREASE,
        "Vote was unsuccessful"
    );
}

#[test]
pub fn jcli_private_vote_invalid_proof() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap().into_persistent();

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let communication_sk = jcli.votes().committee().communication_key().generate();
    let communication_pk = jcli
        .votes()
        .committee()
        .communication_key()
        .to_public(communication_sk)
        .unwrap();
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
        .to_public(member_sk.clone())
        .unwrap();

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

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![alice.to_initial_fund(INITIAL_FUND_PER_WALLET)])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(INITIAL_FUND_PER_WALLET)],
        })
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KesUpdateSpeed::MAXIMUM)
        .with_treasury(INITIAL_TREASURY.into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[alice.to_committee_id()])
        .with_slot_duration(SLOT_DURATION)
        .with_slots_per_epoch(SLOTS_PER_EPOCH)
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
        .seal_with_witness_data(alice.witness_data())
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

    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert).unwrap();

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

#[test]
pub fn private_tally_no_vote_cast() {
    let temp_dir = TempDir::new().unwrap();
    let mut alice = Wallet::default();
    let threshold = 1;

    let private_vote_committee_data_manager =
        CommitteeDataManager::private(&mut OsRng, vec![alice.account_id()], threshold);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(REWARD_INCREASE),
            },
        })
        .private()
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .member_public_keys(private_vote_committee_data_manager.member_public_keys())
        .options_size(3)
        .build();

    let vote_plan_cert = vote_plan_cert(
        &alice,
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        &vote_plan,
    )
    .into();
    let wallets = [&alice];

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.to_initial_fund(INITIAL_FUND_PER_WALLET))
                .collect(),
        )
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![alice.to_initial_token(INITIAL_FUND_PER_WALLET)],
        })
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(SLOTS_PER_EPOCH)
        .with_certs(vec![vote_plan_cert])
        .with_slot_duration(SLOT_DURATION)
        .with_treasury(INITIAL_TREASURY.into())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate::first()
            .next_epoch()
            .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    wait_for_epoch(2, jormungandr.rest());

    let transaction_sender =
        transaction_sender.set_valid_until(chain_impl_mockchain::block::BlockDate {
            epoch: 3,
            slot_id: 0,
        });

    let vote_plan_statuses = jormungandr
        .rest()
        .vote_plan_statuses()
        .unwrap()
        .first()
        .unwrap()
        .clone();

    let decrypted_shares = private_vote_committee_data_manager
        .decrypt_tally(&vote_plan_statuses.into())
        .unwrap();

    let mempool_check = transaction_sender
        .send_private_vote_tally(&mut alice, &vote_plan, decrypted_shares, &jormungandr)
        .unwrap();

    assert!(FragmentVerifier::wait_and_verify_is_in_block(
        Duration::from_secs(5),
        mempool_check,
        &jormungandr,
    )
    .is_ok());

    assert_eq!(
        NodeState::Running,
        jormungandr.rest().stats().unwrap().state
    );

    assert!(jormungandr.check_no_errors_in_log().is_ok());
}
