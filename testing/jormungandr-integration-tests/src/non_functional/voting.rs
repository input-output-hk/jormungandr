use crate::common::jormungandr::{ConfigurationBuilder, Starter};
use assert_fs::TempDir;
use chain_core::property::BlockDate;
use chain_impl_mockchain::{
    certificate::{VoteAction, VoteTallyPayload},
    ledger::governance::TreasuryGovernanceAction,
    value::Value,
};
use chain_impl_mockchain::testing::build_tally_decrypt_share;
use jormungandr_lib::interfaces::InitialUTxO;
use jormungandr_testing_utils::testing::VoteCastsGenerator;
use jormungandr_testing_utils::testing::{
    benchmark_consumption, FragmentStatusProvider, ResourcesUsage, VotePlanBuilder,
};
use jormungandr_testing_utils::{
    testing::{node::time::wait_for_epoch, vote_plan_cert, FragmentSender, FragmentSenderSetup},
    wallet::Wallet,
};
use jortestkit::load::{self, Configuration, Monitor};
use rand::rngs::OsRng;
use chain_impl_mockchain::testing::data::CommitteeMembersManager;

#[test]
pub fn tally_public_vote_load_test() {
    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 10_000;
    let temp_dir = TempDir::new().unwrap();
    let mut rng = OsRng;

    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(1_000)
        .collect();

    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(5, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(6, 0))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(&committee, &vote_plan).into();
    let mut funds: Vec<InitialUTxO> = vec![committee.to_initial_fund(initial_fund_per_wallet)];

    let mut config_builder = ConfigurationBuilder::new();
    for voter in voters.iter() {
        funds.push(voter.to_initial_fund(initial_fund_per_wallet));

        if funds.len() >= 254 {
            config_builder.with_funds(funds.clone());
            funds.clear();
        }
    }

    let config = config_builder
        .with_committees(&[&committee.clone()])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let configuration = Configuration::requests_per_thread(5, 5, 100, Monitor::Standard(100), 100);

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::no_verify(),
    );

    let benchmark_consumption_monitor =
        benchmark_consumption("tallying_public_vote_with_10_000_votes")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process("Node", jormungandr.pid() as usize)
            .start_async(std::time::Duration::from_secs(30));

    let votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );

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

    wait_for_epoch(5, jormungandr.explorer().clone());

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(6, jormungandr.explorer().clone());

    benchmark_consumption_monitor.stop();

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


    assert_eq!(rewards_after,rewards_before + rewards_increase,"rewards not increased");

    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn tally_private_vote_load_test() {
    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 10_000;
    let temp_dir = TempDir::new().unwrap().into_persistent();

    const MEMBERS_NO: usize = 3;
    const THRESHOLD: usize = 2;
    let mut rng = OsRng;

    let members = CommitteeMembersManager::new(&mut rng, THRESHOLD, MEMBERS_NO);

    let committee_keys = members
        .members()
        .iter()
        .map(|committee_member| committee_member.public_key())
        .collect::<Vec<_>>();


    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(1_000)
        .collect();

    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);
    let mut committee_2 = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(1)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(5, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(6, 0))
        .private()
        .member_public_keys(committee_keys)
        .build();

    let vote_plan_cert = vote_plan_cert(&committee, &vote_plan).into();
    let mut funds: Vec<InitialUTxO> = vec![committee.to_initial_fund(initial_fund_per_wallet)];

    let mut config_builder = ConfigurationBuilder::new();
    for voter in voters.iter() {
        funds.push(voter.to_initial_fund(initial_fund_per_wallet));

        if funds.len() >= 254 {
            config_builder.with_funds(funds.clone());
            funds.clear();
        }
    }

    let config = config_builder
        .with_committees(&[&committee.clone()])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(2)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let configuration = Configuration::requests_per_thread(5, 5, 0, Monitor::Standard(100), 100);

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::no_verify(),
    );

    let benchmark_consumption_monitor =
        benchmark_consumption("tallying_public_vote_with_10_000_votes")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process("Node", jormungandr.pid() as usize)
            .start_async(std::time::Duration::from_secs(30));

    let votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );

    let fragment_sender = FragmentSender::new(jormungandr.genesis_block_hash(), jormungandr.fees(), Default::default());
    fragment_sender.send_transaction(&mut committee,&committee_2,&jormungandr,1.into()).unwrap();


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

    wait_for_epoch(5, jormungandr.explorer().clone());

    transaction_sender.send_encrypted_tally(&mut committee,&vote_plan,&jormungandr).unwrap();

    let active_vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    let vote_plan_status = active_vote_plans
        .iter()
        .find(|c_vote_plan| {
            c_vote_plan.id == vote_plan.to_id().into()
        })
        .unwrap();

    let shares = build_tally_decrypt_share(&vote_plan_status.clone().into(), &members);

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Private{ shares },
        )
        .unwrap();

    wait_for_epoch(6, jormungandr.explorer().clone());

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


    assert_eq!(rewards_after,rewards_before + rewards_increase,"rewards not increased");


    benchmark_consumption_monitor.stop();

    jormungandr.assert_no_errors_in_log();
}

#[test]
pub fn tally_private_vote_load_test() {
    let rewards_increase = 10u64;
    let initial_fund_per_wallet = 10_000;
    let temp_dir = TempDir::new().unwrap();
    let mut rng = OsRng;

    const MEMBERS_NO: usize = 3;
    const THRESHOLD: usize = 2;
    let mut rng = OsRng;

    let members = CommitteeMembersManager::new(&mut rng, THRESHOLD, MEMBERS_NO);

    let committee_keys = members
        .members()
        .iter()
        .map(|committee_member| committee_member.public_key())
        .collect::<Vec<_>>();


    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(1_000)
        .collect();

    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(3)
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(rewards_increase),
            },
        })
        .with_vote_start(BlockDate::from_epoch_slot_id(0, 0))
        .with_tally_start(BlockDate::from_epoch_slot_id(10, 0))
        .with_tally_end(BlockDate::from_epoch_slot_id(11, 0))
        .private()
        .member_public_keys(committee_keys)
        .build();

    let vote_plan_cert = vote_plan_cert(&committee, &vote_plan).into();
    let mut funds: Vec<InitialUTxO> = vec![committee.to_initial_fund(initial_fund_per_wallet)];

    let mut config_builder = ConfigurationBuilder::new();
    for voter in voters.iter() {
        funds.push(voter.to_initial_fund(initial_fund_per_wallet));

        if funds.len() >= 254 {
            config_builder.with_funds(funds.clone());
            funds.clear();
        }
    }

    let config = config_builder
        .with_committees(&[&committee.clone()])
        .with_slots_per_epoch(60)
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(1)
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let configuration = Configuration::requests_per_thread(5, 5, 100, Monitor::Standard(100), 100);

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        FragmentSenderSetup::no_verify(),
    );

    let mut benchmark_consumption_monitor =
        benchmark_consumption("tallying_public_vote_with_10_000_votes")
            .target(ResourcesUsage::new(10, 200_000, 5_000_000))
            .for_process("Node", jormungandr.pid() as usize)
            .start_async(std::time::Duration::from_secs(30));

    let mut votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        configuration,
        "Wallet backend load test",
    );

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

    wait_for_epoch(5, jormungandr.explorer().clone());

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(6, jormungandr.explorer().clone());

    benchmark_consumption_monitor.stop().unwrap();

    jormungandr.assert_no_errors_in_log();
}
