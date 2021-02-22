use crate::common::startup::start_stake_pool;
use crate::common::{
    jcli::JCli,
    jormungandr::{ConfigurationBuilder, Starter},
};
use assert_fs::{
    fixture::{FileWriteStr, PathChild},
    TempDir,
};
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    chaintypes::ConsensusType,
    ledger::governance::TreasuryGovernanceAction,
    milli::Milli,
    testing::VoteTestGen,
    value::Value,
    vote::{Choice, CommitteeId},
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

const TEST_COMMITTEE_SIZE: usize = 3;

fn generate_wallets_and_committee<RNG>(rng: &mut RNG) -> (Vec<Wallet>, Vec<CommitteeIdDef>)
where
    RNG: CryptoRng + RngCore,
{
    let mut ids = Vec::new();
    let mut wallets = Vec::new();
    for _i in 0..TEST_COMMITTEE_SIZE {
        let wallet = Wallet::new_account(rng);
        ids.push(wallet.to_committee_id());
        wallets.push(wallet);
    }
    (wallets, ids)
}

#[test]
pub fn test_get_committee_id() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let mut rng = OsRng;
    let (_, mut expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let leader_key_pair = KeyPair::generate(&mut rng);

    let config = ConfigurationBuilder::new()
        .with_leader_key_pair(leader_key_pair.clone())
        .with_committee_ids(expected_committee_ids.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

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

    let mut rng = OsRng;
    let (wallets, expected_committee_ids) = generate_wallets_and_committee(&mut rng);

    let expected_vote_plan = VoteTestGen::vote_plan();

    let vote_plan_cert = vote_plan_cert(&wallets[0], &expected_vote_plan).into();

    let config = ConfigurationBuilder::new()
        .with_committee_ids(expected_committee_ids.clone())
        .with_certs(vec![vote_plan_cert])
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    assert!(vote_plans.len() == 1);

    let vote_plan = vote_plans.get(0).unwrap();
    assert_eq!(
        vote_plan.id.to_string(),
        expected_vote_plan.to_id().to_string()
    );
}
use chain_addr::Discrimination;
use jormungandr_testing_utils::testing::VotePlanBuilder;

#[test]
pub fn test_vote_flow_bft() {
    let favorable_choice = Choice::new(1);

    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 1_000_000;
    let temp_dir = TempDir::new().unwrap();

    let mut rng = OsRng;
    let mut alice = Wallet::new_account(&mut rng);
    let mut bob = Wallet::new_account(&mut rng);
    let mut clarice = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(2, 0))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(&alice, &vote_plan).into();
    let wallets = [&alice, &bob, &clarice];
    let config = ConfigurationBuilder::new()
        .with_funds(
            wallets
                .iter()
                .map(|x| x.to_initial_fund(initial_fund_per_wallet))
                .collect(),
        )
        .with_committees(&wallets)
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::resend_3_times(),
    );

    transaction_sender
        .send_vote_cast(&mut alice, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();
    transaction_sender
        .send_vote_cast(&mut bob, &vote_plan, 0, &favorable_choice, &jormungandr)
        .unwrap();

    let rewards_before = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    wait_for_epoch(1, jormungandr.explorer().clone());

    transaction_sender
        .send_vote_tally(
            &mut clarice,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(2, jormungandr.explorer().clone());

    assert_first_proposal_has_votes(
        2 * initial_fund_per_wallet,
        jormungandr.rest().vote_plan_statuses().unwrap(),
    );

    let rewards_after = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

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
    assert!(proposal.tally.is_some());
    match proposal.tally.as_ref().unwrap() {
        Tally::Public { result } => {
            let results = result.results();
            assert_eq!(*results.get(0).unwrap(), 0);
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

    let mut rng = OsRng;
    let mut alice = Wallet::new_account(&mut rng);
    let mut bob = Wallet::new_account(&mut rng);
    let mut clarice = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(&alice, &vote_plan).into();
    let mut config = ConfigurationBuilder::new();
    config
        .with_committees(&[&alice, &bob, &clarice])
        .with_slots_per_epoch(60)
        .with_consensus_genesis_praos_active_slot_coeff(
            ActiveSlotCoefficient::new(Milli::from_millis(1_000)).unwrap(),
        )
        .with_certs(vec![vote_plan_cert])
        .with_total_rewards_supply(1_000_000.into())
        .with_slot_duration(1);

    let (jormungandr, _stake_pools) = start_stake_pool(
        &[alice.clone()],
        &[bob.clone(), clarice.clone()],
        &mut config,
    )
    .unwrap();

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
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

    wait_for_epoch(1, jormungandr.explorer().clone());

    transaction_sender
        .send_vote_tally(
            &mut alice,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(3, jormungandr.explorer().clone());

    let rewards_after = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

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
        .with_vote_start(BlockDate::from_epoch_slot_id(1, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(2, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(3, 0))
        .public()
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
        .with_treasury(1000.into())
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

    let rewards_before = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    time::wait_for_epoch(1, jormungandr.explorer());

    let vote_plan_id = jcli.certificate().vote_plan_id(&vote_plan_cert);
    let vote_cast = jcli
        .certificate()
        .new_public_vote_cast(vote_plan_id.clone(), 0, yes_choice);

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

    let vote_tally_cert = jcli.certificate().new_public_vote_tally(vote_plan_id);

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

    let rewards_after = jormungandr
        .explorer()
        .status()
        .unwrap()
        .data
        .unwrap()
        .status
        .latest_block
        .treasury
        .unwrap()
        .rewards
        .parse::<u64>()
        .unwrap();

    // We want to make sure that our small rewards increase is reflexed in current rewards amount
    assert!(
        rewards_after == rewards_before + rewards_increase,
        "Vote was unsuccessful"
    );
}
