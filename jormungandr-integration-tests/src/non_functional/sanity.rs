#![cfg(feature = "sanity-non-functional")]
use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, JormungandrProcess},
    startup,
};

use super::NodeStuckError;

use jormungandr_lib::{
    interfaces::{ActiveSlotCoefficient, KESUpdateSpeed, Value},
    testing::{
        benchmark_efficiency, benchmark_endurance, EfficiencyBenchmarkDef,
        EfficiencyBenchmarkFinish, Endurance, Thresholds,
    },
    wallet::Wallet,
};
use std::{iter, time::Duration};

#[test]
pub fn test_100_transaction_is_processed_in_10_packs_to_many_accounts() {
    let receivers: Vec<Wallet> = iter::from_fn(|| Some(startup::create_new_account_address()))
        .take(10)
        .collect();
    send_and_measure_100_transaction_in_10_packs_for_recievers(
        receivers,
        "100_transaction_are_processed_in_10_packs_to_many_accounts",
    );
}

#[test]
pub fn test_100_transaction_is_processed_in_10_packs_to_single_account() {
    let single_reciever = startup::create_new_account_address();
    let receivers: Vec<Wallet> = iter::from_fn(|| Some(single_reciever.clone()))
        .take(10)
        .collect();
    send_and_measure_100_transaction_in_10_packs_for_recievers(
        receivers,
        "100_transaction_are_processed_in_10_packs_to_single_account",
    );
}

fn send_and_measure_100_transaction_in_10_packs_for_recievers(receivers: Vec<Wallet>, info: &str) {
    let pack_size = 10;
    let target = (pack_size * receivers.len()) as u32;
    let efficiency_benchmark_result = send_100_transaction_in_10_packs_for_recievers(
        pack_size,
        receivers,
        benchmark_efficiency(info.to_owned()).target(target),
    );
    efficiency_benchmark_result.print();
}

fn send_100_transaction_in_10_packs_for_recievers(
    iterations_count: usize,
    receivers: Vec<Wallet>,
    efficiency_benchmark_def: &mut EfficiencyBenchmarkDef,
) -> EfficiencyBenchmarkFinish {
    let mut sender = startup::create_new_account_address();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(2)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;
    let mut efficiency_benchmark_run = efficiency_benchmark_def.start();
    for i in 0..iterations_count {
        let transation_messages: Vec<String> = receivers
            .iter()
            .map(|receiver| {
                let message =
                    JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                        .assert_add_account(&sender.address().to_string(), &output_value.into())
                        .assert_add_output(&receiver.address().to_string(), &output_value.into())
                        .assert_finalize()
                        .seal_with_witness_for_address(&sender)
                        .assert_to_message();
                sender.confirm_transaction();
                message
            })
            .collect();

        println!("Sending pack of 10 transaction no. {}", i);
        if let Err(err) = super::send_transaction_and_ensure_block_was_produced(
            &transation_messages,
            &jormungandr,
        ) {
            return efficiency_benchmark_run.exception(err.to_string());
        }

        efficiency_benchmark_run.increment_by(receivers.len() as u32);
    }
    efficiency_benchmark_run.stop()
}

#[test]
pub fn test_100_transaction_is_processed_simple() {
    let transaction_max_count = 100;
    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;
    let mut benchmark = benchmark_efficiency("test_100_transaction_is_processed_simple")
        .target(transaction_max_count)
        .start();

    for i in 0..transaction_max_count {
        let transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address().to_string(), &output_value.into())
                .assert_add_output(&receiver.address().to_string(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();
        println!("Sending transaction no. {}", i + 1);

        if let Err(error) = check_transaction_was_processed(
            transaction.to_owned(),
            &receiver,
            i.into(),
            &jormungandr,
        ) {
            let message = format!("{}", error);
            benchmark.exception(message).print();
            return;
        }

        benchmark.increment();
    }
    benchmark.stop().print();
    jcli_wrapper::check_all_transaction_log_shows_in_block(&jormungandr);
}

fn check_transaction_was_processed(
    transaction: String,
    receiver: &Wallet,
    i: u64,
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    super::send_transaction_and_ensure_block_was_produced(&vec![transaction], &jormungandr)?;

    check_funds_transferred_to(
        &receiver.address().to_string(),
        (i + 1).into(),
        &jormungandr,
    )?;

    jormungandr
        .check_no_errors_in_log()
        .map_err(|err| NodeStuckError::InternalJormungandrError(err))
}

fn check_funds_transferred_to(
    address: &str,
    value: Value,
    jormungandr: &JormungandrProcess,
) -> Result<(), NodeStuckError> {
    let account_state =
        jcli_wrapper::assert_rest_account_get_stats(address, &jormungandr.rest_address());

    if *account_state.value() != value {
        return Err(NodeStuckError::FundsNotTransfered {
            actual: account_state.value().clone(),
            expected: value.clone(),
            logs: jormungandr.logger.get_log_content(),
        });
    }
    Ok(())
}

#[test]
pub fn test_blocks_are_being_created_for_more_than_15_minutes() {
    let mut sender = startup::create_new_account_address();
    let mut receiver = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        &[],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;
    let benchmark = benchmark_endurance("test_blocks_are_created_for_more_than_15_minutes")
        .target(Duration::from_secs(900))
        .start();

    loop {
        let transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address().to_string(), &output_value.into())
                .assert_add_output(&receiver.address().to_string(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();
        if let Err(err) =
            super::send_transaction_and_ensure_block_was_produced(&vec![transaction], &jormungandr)
        {
            // temporary threshold for the time issue with transaction stuck is resolved
            let temporary_threshold =
                Thresholds::<Endurance>::new_endurance(Duration::from_secs(400));
            benchmark
                .exception(err.to_string())
                .print_with_thresholds(temporary_threshold);
            return;
        }

        if benchmark.max_endurance_reached() {
            benchmark.stop().print();
            return;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}
