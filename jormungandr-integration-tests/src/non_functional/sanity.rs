#![cfg(feature = "sanity-non-functional")]
use crate::common::{
    data::address::Account,
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::ConfigurationBuilder,
    startup,
};
use jormungandr_lib::interfaces::{ActiveSlotCoefficient, KESUpdateSpeed, Value};
use std::iter;
use std::time::SystemTime;

#[test]
pub fn test_100_transaction_is_processed_in_10_packs() {
    let receivers: Vec<Account> = iter::from_fn(|| Some(startup::create_new_account_address()))
        .take(10)
        .collect();
    send_100_transaction_in_10_packs_for_recievers(10, receivers)
}

#[test]
pub fn test_100_transaction_is_processed_in_10_packs_to_single_account() {
    let single_reciever = startup::create_new_account_address();
    let receivers: Vec<Account> = iter::from_fn(|| Some(single_reciever.clone()))
        .take(1)
        .collect();
    send_100_transaction_in_10_packs_for_recievers(10, receivers)
}

fn send_100_transaction_in_10_packs_for_recievers(
    iterations_count: usize,
    receivers: Vec<Account>,
) {
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

    let transation_messages: Vec<String> = receivers
        .iter()
        .map(|receiver| {
            let message =
                JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                    .assert_add_account(&sender.address.clone(), &output_value.into())
                    .assert_add_output(&receiver.address.clone(), &output_value.into())
                    .assert_finalize()
                    .seal_with_witness_for_address(&sender)
                    .assert_to_message();
            sender.confirm_transaction();
            message
        })
        .collect();

    for _ in 0..iterations_count {
        super::send_transaction_and_ensure_block_was_produced(&transation_messages, &jormungandr);
    }
}

#[test]
pub fn test_100_transaction_is_processed() {
    let mut sender = startup::create_new_account_address();
    let mut receiver = startup::create_new_account_address();

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

    for i in 0..100 {
        let transaction =
            JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash)
                .assert_add_account(&sender.address.clone(), &output_value.into())
                .assert_add_output(&receiver.address.clone(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();

        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr.rest_address());

        assert_funds_transferred_to(&receiver.address, i.into(), &jormungandr.rest_address());
        jormungandr.assert_no_errors_in_log();
        std::mem::swap(&mut sender, &mut receiver);
    }

    jcli_wrapper::assert_all_transaction_log_shows_in_block(&jormungandr.rest_address());
}

fn assert_funds_transferred_to(address: &str, value: Value, host: &str) {
    let account_state = jcli_wrapper::assert_rest_account_get_stats(address, host);

    assert_eq!(
        *account_state.value(),
        value,
        "funds were transfer on wrong account (or didn't at all). AccountState: {:?}, expected funds : {:?}",account_state,value
    );
}

#[test]
pub fn test_blocks_are_being_created_for_more_than_15_minutes() {
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
                .assert_add_account(&sender.address.clone(), &output_value.into())
                .assert_add_output(&receiver.address.clone(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message();

        sender.confirm_transaction();

        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr.rest_address());

        // 900 s = 15 minutes
        if now.elapsed().unwrap().as_secs() > 900 {
            break;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}
