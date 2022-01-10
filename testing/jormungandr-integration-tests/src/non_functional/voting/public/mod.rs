mod load;
mod noise;
#[cfg(feature = "soak")]
mod soak;

use crate::non_functional::voting::config::PublicVotingLoadTestConfig;
use assert_fs::TempDir;
use chain_core::property::BlockDate as _;
use chain_impl_mockchain::{
    block::BlockDate,
    certificate::{VoteAction, VoteTallyPayload},
    ledger::governance::TreasuryGovernanceAction,
    value::Value,
};
use jormungandr_testing_utils::testing::jormungandr::{ConfigurationBuilder, Starter};
use jormungandr_testing_utils::testing::node::time::wait_for_epoch;
use jormungandr_testing_utils::testing::{benchmark_consumption, VotePlanBuilder};
use jortestkit::load::Configuration;
use jortestkit::measurement::Status;
use loki::{AdversaryFragmentSender, AdversaryFragmentSenderSetup};
use mjolnir::generators::{AdversaryFragmentGenerator, FragmentStatusProvider, VoteCastsGenerator};
use rand::rngs::OsRng;
use thor::{vote_plan_cert, BlockDateGenerator, FragmentSender, FragmentSenderSetup, Wallet};

pub fn public_vote_load_scenario(quick_config: PublicVotingLoadTestConfig) {
    let temp_dir = TempDir::new().unwrap();
    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);

    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(quick_config.wallets_count())
        .collect();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(quick_config.proposals_count())
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(quick_config.rewards_increase()),
            },
        })
        .vote_start(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[0],
            0,
        ))
        .tally_start(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[1],
            0,
        ))
        .tally_end(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[2],
            0,
        ))
        .public()
        .build();

    let vote_plan_cert = vote_plan_cert(
        &committee,
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        &vote_plan,
    )
    .into();

    let config = ConfigurationBuilder::new()
        .with_fund(committee.to_initial_fund(quick_config.initial_fund_per_wallet()))
        .with_funds_split_if_needed(
            voters
                .iter()
                .map(|x| x.to_initial_fund(quick_config.initial_fund_per_wallet()))
                .collect(),
        )
        .with_committees(&[committee.to_committee_id()])
        .with_slots_per_epoch(quick_config.slots_in_epoch())
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(quick_config.slot_duration())
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();
    let block_date_generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        false,
    );

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        block_date_generator,
        FragmentSenderSetup::no_verify(),
    );

    let benchmark_consumption_monitor = benchmark_consumption(quick_config.measurement_name())
        .target(quick_config.target_resources_usage())
        .for_process("Node", jormungandr.pid() as usize)
        .start_async(std::time::Duration::from_secs(30));

    let votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    let stats = jortestkit::load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        quick_config.configuration(),
        &quick_config.measurement_name(),
    );

    stats.print_summary(&quick_config.measurement_name());
    assert_eq!(
        stats
            .measure(
                &quick_config.measurement_name(),
                quick_config.tx_target_success_rate()
            )
            .status(),
        Status::Green
    );

    wait_for_epoch(quick_config.voting_timing()[1], jormungandr.rest());

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(quick_config.voting_timing()[2], jormungandr.rest());

    let active_vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    let vote_plan_status = active_vote_plans
        .iter()
        .find(|c_vote_plan| c_vote_plan.id == vote_plan.to_id().into())
        .unwrap();

    for proposal in vote_plan_status.proposals.iter() {
        assert!(
            proposal.tally.is_some(),
            "Proposal is not tallied {:?}",
            proposal
        );
    }

    benchmark_consumption_monitor.stop();

    jormungandr.assert_no_errors_in_log();
}

pub fn adversary_public_vote_load_scenario(
    quick_config: PublicVotingLoadTestConfig,
    adversary_noise_config: Configuration,
) {
    let temp_dir = TempDir::new().unwrap();
    let mut rng = OsRng;
    let mut committee = Wallet::new_account(&mut rng);

    let mut noise_wallet_from = Wallet::new_account(&mut rng);

    let voters: Vec<Wallet> = std::iter::from_fn(|| Some(Wallet::new_account(&mut rng)))
        .take(quick_config.wallets_count())
        .collect();

    let vote_plan = VotePlanBuilder::new()
        .proposals_count(quick_config.proposals_count())
        .action_type(VoteAction::Treasury {
            action: TreasuryGovernanceAction::TransferToRewards {
                value: Value(quick_config.rewards_increase()),
            },
        })
        .vote_start(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[0],
            0,
        ))
        .tally_start(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[1],
            0,
        ))
        .tally_end(BlockDate::from_epoch_slot_id(
            quick_config.voting_timing()[2],
            0,
        ))
        .build();

    let vote_plan_cert = vote_plan_cert(
        &committee,
        chain_impl_mockchain::block::BlockDate::first().next_epoch(),
        &vote_plan,
    )
    .into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![
            noise_wallet_from.to_initial_fund(1_000_000_000),
            committee.to_initial_fund(quick_config.initial_fund_per_wallet()),
        ])
        .with_funds_split_if_needed(
            voters
                .iter()
                .map(|x| x.to_initial_fund(quick_config.initial_fund_per_wallet()))
                .collect(),
        )
        .with_committees(&[committee.to_committee_id()])
        .with_slots_per_epoch(quick_config.slots_in_epoch())
        .with_certs(vec![vote_plan_cert])
        .with_explorer()
        .with_slot_duration(quick_config.slot_duration())
        .with_block_content_max_size(quick_config.block_content_max_size().into())
        .with_treasury(1_000.into())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config)
        .start()
        .unwrap();

    let settings = jormungandr.rest().settings().unwrap();

    let generator = BlockDateGenerator::rolling(
        &settings,
        BlockDate {
            epoch: 1,
            slot_id: 0,
        },
        false,
    );

    let transaction_sender = FragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        generator,
        FragmentSenderSetup::no_verify(),
    );

    let adversary_transaction_sender = AdversaryFragmentSender::new(
        jormungandr.genesis_block_hash(),
        jormungandr.fees(),
        chain_impl_mockchain::block::BlockDate {
            epoch: 1,
            slot_id: 0,
        }
        .into(),
        AdversaryFragmentSenderSetup::no_verify(),
    );

    let benchmark_consumption_monitor = benchmark_consumption(&quick_config.measurement_name())
        .target(quick_config.target_resources_usage())
        .for_process("Node", jormungandr.pid() as usize)
        .start_async(std::time::Duration::from_secs(30));

    let mut adversary_votes_generator = AdversaryFragmentGenerator::new(
        jormungandr.to_remote(),
        transaction_sender.clone(),
        adversary_transaction_sender,
    );

    adversary_votes_generator.fill_from_faucet(&mut noise_wallet_from);

    let _noise = jortestkit::load::start_background_async(
        adversary_votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        adversary_noise_config,
        "noise fragments",
    );

    let votes_generator = VoteCastsGenerator::new(
        voters,
        vote_plan.clone(),
        jormungandr.to_remote(),
        transaction_sender.clone(),
    );

    let stats = jortestkit::load::start_async(
        votes_generator,
        FragmentStatusProvider::new(jormungandr.to_remote()),
        quick_config.configuration(),
        &quick_config.measurement_name(),
    );

    stats.print_summary(&quick_config.measurement_name());
    assert_eq!(
        stats
            .measure(
                &quick_config.measurement_name(),
                quick_config.tx_target_success_rate()
            )
            .status(),
        Status::Green
    );

    wait_for_epoch(quick_config.voting_timing()[1], jormungandr.rest());

    transaction_sender
        .send_vote_tally(
            &mut committee,
            &vote_plan,
            &jormungandr,
            VoteTallyPayload::Public,
        )
        .unwrap();

    wait_for_epoch(quick_config.voting_timing()[2], jormungandr.rest());

    let active_vote_plans = jormungandr.rest().vote_plan_statuses().unwrap();
    let vote_plan_status = active_vote_plans
        .iter()
        .find(|c_vote_plan| c_vote_plan.id == vote_plan.to_id().into())
        .unwrap();

    for proposal in vote_plan_status.proposals.iter() {
        assert!(
            proposal.tally.is_some(),
            "Proposal is not tallied {:?}",
            proposal
        );
    }

    benchmark_consumption_monitor.stop();

    jormungandr.assert_no_errors_in_log();
}
