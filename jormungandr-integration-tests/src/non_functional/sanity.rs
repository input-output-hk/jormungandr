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
        thresholds_for_transaction_counter, thresholds_for_transaction_endurance, Measurement,
    },
    wallet::Wallet,
};
use std::iter;
use std::time::{Duration, SystemTime};

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
    let pack_size = 2;
    let thresholds = thresholds_for_transaction_counter((pack_size * receivers.len()) as u64);
    let sucessfully_tx_sent_counter =
        send_100_transaction_in_10_packs_for_recievers(pack_size, receivers) as u64;
    println!(
        "{}",
        Measurement::new(info.to_owned(), sucessfully_tx_sent_counter, thresholds)
    )
}

fn send_100_transaction_in_10_packs_for_recievers(
    iterations_count: usize,
    receivers: Vec<Wallet>,
) -> usize {
    let mut sender = startup::create_new_account_address();
    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(2)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;
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
            println!("Test finished prematurely, due to: {}", err.to_string());
            return i * receivers.len();
        }
    }
    iterations_count * receivers.len()
}

#[test]
pub fn test_100_transaction_is_processed_simple() {
    let transaction_max_count = 100;
    let measurement_name = "test_100_transaction_is_processed_simple";
    let thresholds = thresholds_for_transaction_counter(transaction_max_count as u64);

    let mut sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let output_value = 1 as u64;

    for i in 0..transaction_max_count {
        let transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address().to_string(), &output_value.into())
                .assert_add_output(&receiver.address().to_string(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();
        println!("Sending transaction no. {}", i);

        if let Err(error) =
            check_transaction_was_processed(transaction.to_owned(), &receiver, i, &jormungandr)
        {
            println!("Test finished prematurely, due to: {}", error.to_string());
            println!(
                "{}",
                Measurement::new(measurement_name.to_owned(), i, thresholds)
            );
            return;
        }
    }
    println!(
        "{}",
        Measurement::new(
            measurement_name.to_owned(),
            transaction_max_count,
            thresholds
        )
    );
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
    let measurement_name = "test_blocks_are_created_for_more_than_15_minutes";
    let test_endurance = 900; // 900 s = 15 minutes
    let thresholds = thresholds_for_transaction_endurance(test_endurance);

    let mut sender = startup::create_new_account_address();
    let mut receiver = startup::create_new_account_address();

    let (jormungandr, _) = startup::start_stake_pool(
        &[sender.clone()],
        ConfigurationBuilder::new()
            .with_slots_per_epoch(60)
            .with_consensus_genesis_praos_active_slot_coeff(ActiveSlotCoefficient::MAXIMUM)
            .with_slot_duration(4)
            .with_epoch_stability_depth(10)
            .with_kes_update_speed(KESUpdateSpeed::new(43200).unwrap()),
    )
    .unwrap();

    let now = SystemTime::now();
    let output_value = 1 as u64;

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
            println!("Test finished prematurely, due to: {}", err.to_string());
            println!(
                "{}",
                Measurement::new(
                    measurement_name.to_owned(),
                    now.elapsed().unwrap().into(),
                    thresholds
                )
            );
            return;
        }

        if now.elapsed().unwrap().as_secs() > test_endurance {
            break;
        }
        std::mem::swap(&mut sender, &mut receiver);
    }
    println!(
        "{}",
        Measurement::new(
            measurement_name.to_owned(),
            Duration::from_secs(test_endurance).into(),
            thresholds
        )
    );
}
