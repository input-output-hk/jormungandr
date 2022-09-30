use crate::{startup, startup::start_stake_pool};
use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    chaintypes::ConsensusType,
    fee::LinearFee,
    ledger::governance::TreasuryGovernanceAction,
    milli::Milli,
    testing::VoteTestGen,
    tokens::minting_policy::MintingPolicy,
    value::Value,
    vote::{Choice, CommitteeId},
};
use jormungandr_automation::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
    testing::{
        asserts::VotePlanStatusAssert,
        time::{self, wait_for_epoch},
        VotePlanExtension,
    },
};
use jormungandr_lib::{
    crypto::key::KeyPair,
    interfaces::{
        AccountVotes, ActiveSlotCoefficient, BlockDate as BlockDateDto, CommitteeIdDef, FeesGoTo,
        InitialToken, KesUpdateSpeed, Tally, VotePlanStatus,
    },
};
use rand::rngs::OsRng;
use std::time::Duration;
use thor::{vote_plan_cert, FragmentSender, FragmentSenderSetup, Wallet};

const TEST_COMMITTEE_SIZE: usize = 3;

fn generate_wallets_and_committee() -> (Vec<Wallet>, Vec<CommitteeIdDef>) {
    let mut ids = Vec::new();
    let mut wallets = Vec::new();
    for _i in 0..TEST_COMMITTEE_SIZE {
        let wallet = Wallet::default();
        ids.push(wallet.to_committee_id());
        wallets.push(wallet);
    }
    (wallets, ids)
}

#[test]
pub fn test_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let rng = OsRng;
    let (_, mut expected_committee_ids) = generate_wallets_and_committee();

    let leader_key_pair = KeyPair::generate(rng);

    let config = ConfigurationBuilder::new()
        .with_leader_key_pair(leader_key_pair.clone())
        .with_committee_ids(expected_committee_ids.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    expected_committee_ids.insert(
        0,
        CommitteeIdDef::from(CommitteeId::from(
            leader_key_pair.identifier().into_public_key(),
        )),
    );

    let actual_committee_ids = jcli
        .rest()
        .v0()
        .vote()
        .active_voting_committees(&jormungandr.rest_uri());

    assert_eq!(expected_committee_ids, actual_committee_ids);
}

#[test]
pub fn test_get_initial_vote_plan() {
    let temp_dir = TempDir::new().unwrap();

    let (wallets, expected_committee_ids) = generate_wallets_and_committee();

    let expected_vote_plan = VoteTestGen::vote_plan();

    let vote_plan_cert = vote_plan_cert(
        &wallets[0],
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        &expected_vote_plan,
    )
    .into();

    let config = ConfigurationBuilder::new()
        .with_committee_ids(expected_committee_ids)
        .with_certs(vec![vote_plan_cert])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    assert!(vote_plans.len() == 1);

    let vote_plan = vote_plans.get(0).unwrap();
    assert_eq!(
        vote_plan.id.to_string(),
        expected_vote_plan.to_id().to_string()
    );
}
use chain_addr::Discrimination;
use jormungandr_automation::testing::VotePlanBuilder;

#[test]
pub fn test_vote_flow_bft() {
    let favorable_choice = Choice::new(1);

    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 1_000_000;
    let temp_dir = TempDir::new().unwrap();

    let mut alice = Wallet::default();
    let mut bob = Wallet::default();
    let mut clarice = Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
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
    let wallets = [&alice, &bob, &clarice];

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.to_initial_fund(initial_fund_per_wallet))
                .collect(),
        )
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![
                alice.to_initial_token(initial_fund_per_wallet),
                bob.to_initial_token(initial_fund_per_wallet),
                clarice.to_initial_token(initial_fund_per_wallet),
            ],
        })
        .with_committees(&[
            alice.to_committee_id(),
            bob.to_committee_id(),
            clarice.to_committee_id(),
        ])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_slot_duration(1)
        .with_treasury(1_000.into())
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

    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut bob, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();

    let rewards_before: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    wait_for_epoch(1, jormungandr.rest());

    let transaction_sender =
        transaction_sender.set_valid_until(chain_impl_mockchain::block::BlockDate {
            epoch: 2,
            slot_id: 0,
        });

    assert_eq!(
        vec![0],
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![0]
        }],
        jormungandr
            .rest()
            .account_votes(alice.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![0],
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), bob.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![0]
        }],
        jormungandr
            .rest()
            .account_votes(bob.address())
            .unwrap()
            .unwrap()
    );

    transaction_sender
        .send_vote_tally(
            &mut clarice,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(2, jormungandr.rest());

    assert_first_proposal_has_votes(
        2 * initial_fund_per_wallet,
        jormungandr.rest().vote_plan_statuses().unwrap(),
    );

    let rewards_after: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    assert!(
        rewards_after == (rewards_before + rewards_increase),
        "Vote was unsuccessful"
    )
}

fn assert_first_proposal_has_votes(stake: u64, vote_plan_statuses: Vec<VotePlanStatus>) {
    println!("{:?}", vote_plan_statuses);
    let proposal = vote_plan_statuses
        .first()
        .unwrap()
        .proposals
        .first()
        .unwrap();
    match &proposal.tally {
        Tally::Public { result } => {
            let results = result.results();
            assert_eq!(*results.first().unwrap(), 0);
            assert_eq!(*results.get(1).unwrap(), stake);
            assert_eq!(*results.get(2).unwrap(), 0);
        }
        Tally::Private { .. } => unimplemented!("Private tally testing is not implemented"),
    }
}

#[test]
pub fn test_vote_flow_praos() {
    let yes_choice = Choice::new(1);
    let no_choice = Choice::new(2);
    let rewards_increase = 10;

    let mut alice = Wallet::default();
    let mut bob = Wallet::default();
    let mut clarice = Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .public()
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

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let mut config = ConfigurationBuilder::new();
    config
        .with_committees(&[
            alice.to_committee_id(),
            bob.to_committee_id(),
            clarice.to_committee_id(),
        ])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![
                alice.to_initial_token(1_000_000),
                bob.to_initial_token(1_000_000),
                clarice.to_initial_token(1_000_000),
            ],
        })
        .with_slots_per_epoch(60)
        .with_consensus_genesis_praos_active_slot_coeff(
            ActiveSlotCoefficient::new(Milli::from_millis(1_000)).unwrap(),
        )
        .with_certs(vec![vote_plan_cert])
        .with_total_rewards_supply(1_000_000.into());

    let (jormungandr, _stake_pools) = start_stake_pool(
        &[alice.clone()],
        &[bob.clone(), clarice.clone()],
        &mut config,
    )
    .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate::first()
            .next_epoch()
            .into(),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &yes_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut bob, &vote_plan, 0, &yes_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut clarice, &vote_plan, 0, &no_choice, &jormungandr)
        .unwrap();

    wait_for_epoch(1, jormungandr.rest());

    let transaction_sender =
        transaction_sender.set_valid_until(chain_impl_mockchain::block::BlockDate {
            epoch: 2,
            slot_id: 0,
        });

    assert_eq!(
        vec![0],
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), alice.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![0]
        }],
        jormungandr
            .rest()
            .account_votes(alice.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![0],
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), bob.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![0]
        }],
        jormungandr
            .rest()
            .account_votes(bob.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![0],
        jormungandr
            .rest()
            .account_votes_with_plan_id(vote_plan.to_id().into(), clarice.address())
            .unwrap()
            .unwrap()
    );
    assert_eq!(
        vec![AccountVotes {
            vote_plan_id: vote_plan.to_id().into(),
            votes: vec![0]
        }],
        jormungandr
            .rest()
            .account_votes(clarice.address())
            .unwrap()
            .unwrap()
    );

    transaction_sender
        .send_vote_tally(
            &mut alice,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(3, jormungandr.rest());

    let rewards_after: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    // We want to make sure that our small rewards increase is reflexed in current rewards amount
    assert!(
        rewards_after
            .to_string()
            .ends_with(&rewards_increase.to_string()),
        "Vote was unsuccessful"
    );
}

#[test]
pub fn jcli_e2e_flow() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();
    let rewards_increase = 10;
    let yes_choice = Choice::new(1);

    let mut rng = OsRng;
    let mut alice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let bob = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);
    let clarice = Wallet::new_account_with_discrimination(&mut rng, Discrimination::Production);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .public()
        .build();

    let vote_plan_json = temp_dir.child("vote_plan.json");
    vote_plan_json.write_str(&vote_plan.as_json_str()).unwrap();

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            alice.to_initial_fund(1_000_000),
            bob.to_initial_fund(1_000_000),
            clarice.to_initial_fund(1_000_000),
        ])
        .with_token(InitialToken {
            token_id: token_id.clone().into(),
            policy: minting_policy.into(),
            to: vec![
                alice.to_initial_token(1_000_000),
                bob.to_initial_token(1_000_000),
                clarice.to_initial_token(1_000_000),
            ],
        })
        .with_block0_consensus(ConsensusType::Bft)
        .with_kes_update_speed(KesUpdateSpeed::new(43200).unwrap())
        .with_fees_go_to(FeesGoTo::Rewards)
        .with_treasury(Value::zero().into())
        .with_total_rewards_supply(Value::zero().into())
        .with_discrimination(Discrimination::Production)
        .with_committees(&[alice.to_committee_id()])
        .with_slots_per_epoch(60)
        .with_consensus_genesis_praos_active_slot_coeff(
            ActiveSlotCoefficient::new(Milli::from_millis(100)).unwrap(),
        )
        .with_treasury(1000.into())
        .with_slot_duration(4)
        .with_slots_per_epoch(10)
        .build(&temp_dir);

    let alice_sk = temp_dir.child("alice_sk");
    alice.save_to_path(alice_sk.path()).unwrap();

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let vote_plan_cert = jcli.certificate().new_vote_plan(vote_plan_json.path());

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
    let vote_cast = jcli
        .certificate()
        .new_public_vote_cast(vote_plan_id.clone(), 0, yes_choice);

    let tx = jcli
        .transaction_builder(jormungandr.genesis_block_hash())
        .new_transaction()
        .add_account(&alice.address().to_string(), &Value::zero().into())
        .add_certificate(&vote_cast)
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
        .add_certificate(&vote_cast)
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
        .add_certificate(&vote_cast)
        .set_expiry_date(BlockDateDto::new(2, 0))
        .finalize()
        .seal_with_witness_data(clarice.witness_data())
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&tx)
        .assert_in_block();

    time::wait_for_epoch(2, jormungandr.rest());

    let vote_tally_cert = jcli.certificate().new_public_vote_tally(vote_plan_id);

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

    let rewards_after: u64 = jormungandr.rest().remaining_rewards().unwrap().into();

    // We want to make sure that our small rewards increase is reflexed in current rewards amount
    assert!(
        rewards_after == rewards_before + rewards_increase,
        "Vote was unsuccessful"
    );
}

#[test]
pub fn duplicated_vote() {
    let mut alice = thor::Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .public()
        .build();

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0))
            .with_token(InitialToken {
                token_id: token_id.clone().into(),
                policy: minting_policy.into(),
                to: vec![alice.to_initial_token(1_000_000_000)],
            }),
    )
    .unwrap();

    let alice_account_state = jormungandr
        .rest()
        .account_state(&alice.account_id())
        .unwrap();

    thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    )
    .send_vote_plan(&mut alice, &vote_plan)
    .unwrap()
    .and_verify_is_in_block(Duration::from_secs(2))
    .unwrap()
    .then_wait_for_epoch(1)
    .cast_vote(&mut alice, &vote_plan, 0, &Choice::new(1))
    .unwrap()
    .and_verify_is_in_block(Duration::from_secs(2))
    .unwrap()
    .cast_vote(&mut alice, &vote_plan, 0, &Choice::new(1))
    .unwrap()
    .and_verify_is_rejected(Duration::from_secs(2))
    .unwrap()
    .update_wallet(&mut alice, &|alice: &mut Wallet| alice.decrement_counter())
    .then_wait_for_epoch(2)
    .tally_vote(&mut alice, &vote_plan, VoteTallyPayload::Public)
    .unwrap()
    .and_verify_is_in_block(Duration::from_secs(2))
    .unwrap();

    let vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    vote_plans.assert_proposal_tally(
        vote_plan.to_id().to_string(),
        0,
        vec![0, (*alice_account_state.value()).into(), 0],
    );
}

#[test]
pub fn non_duplicated_vote() {
    let mut alice = thor::Wallet::default();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .public()
        .build();

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_token(InitialToken {
                token_id: token_id.clone().into(),
                policy: minting_policy.into(),
                to: vec![alice.to_initial_token(1_000_000_000)],
            })
            .with_slots_per_epoch(20)
            .with_slot_duration(3)
            .with_linear_fees(LinearFee::new(0, 0, 0)),
    )
    .unwrap();

    let alice_account_state = jormungandr
        .rest()
        .account_state(&alice.account_id())
        .unwrap();

    let fragment_sender_chain = thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    );

    fragment_sender_chain
        .send_vote_plan(&mut alice, &vote_plan)
        .unwrap()
        .and_verify_is_in_block(Duration::from_secs(2))
        .unwrap()
        .then_wait_for_epoch(1)
        .cast_vote(&mut alice, &vote_plan, 0, &Choice::new(1))
        .unwrap()
        .and_verify_is_in_block(Duration::from_secs(2))
        .unwrap()
        .cast_vote(&mut alice, &vote_plan, 1, &Choice::new(1))
        .unwrap()
        .and_verify_is_in_block(Duration::from_secs(2))
        .unwrap()
        .then_wait_for_epoch(2)
        .tally_vote(&mut alice, &vote_plan, VoteTallyPayload::Public)
        .unwrap()
        .and_verify_is_in_block(Duration::from_secs(2))
        .unwrap();

    let vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    vote_plans.assert_proposal_tally(
        vote_plan.to_id().to_string(),
        0,
        vec![0, (*alice_account_state.value()).into(), 0],
    );
    vote_plans.assert_proposal_tally(
        vote_plan.to_id().to_string(),
        1,
        vec![0, (*alice_account_state.value()).into(), 0],
    );
}

#[test]
pub fn vote_outside_of_choices_is_rejected_in_tally() {
    let mut alice = thor::Wallet::default();
    let options_size = 2;

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .options_size(options_size)
        .action_type(VoteAction::OffChain)
        .vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .public()
        .build();

    let minting_policy = MintingPolicy::new();
    let token_id = vote_plan.voting_token();

    let jormungandr = startup::start_bft(
        vec![&alice],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(10)
            .with_slot_duration(2)
            .with_linear_fees(LinearFee::new(0, 0, 0))
            .with_token(InitialToken {
                token_id: token_id.clone().into(),
                policy: minting_policy.into(),
                to: vec![alice.to_initial_token(1_000_000_000)],
            }),
    )
    .unwrap();

    thor::FragmentChainSender::from_with_setup(
        jormungandr.block0_configuration(),
        jormungandr.to_remote(),
        FragmentSenderSetup::no_verify(),
    )
    .send_vote_plan(&mut alice, &vote_plan)
    .unwrap()
    .and_verify_is_in_block(Duration::from_secs(2))
    .unwrap()
    .then_wait_for_epoch(1)
    .cast_vote(&mut alice, &vote_plan, 0, &Choice::new(options_size))
    .unwrap()
    .and_verify_is_rejected_with_message(Duration::from_secs(2), "Invalid option choice")
    .unwrap();
}
