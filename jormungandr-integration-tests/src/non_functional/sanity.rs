#![cfg(feature = "sanity-non-functional")]
use crate::common::{
    configuration::genesis_model::Fund,
    data::address::Account,
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

use jormungandr_lib::interfaces::UTxOInfo;
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

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 10000000.into(),
        }])
        .with_slot_duration(2)
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let output_value = 1 as u64;

    let transation_messages: Vec<String> = receivers
        .iter()
        .map(|receiver| {
            let message = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
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
    let mut sender = startup::create_new_utxo_address();
    let mut receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();
    let mut utxo = config.block0_utxo_for_address(&sender);
    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    for _i in 0..100 {
        let transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address.clone(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();

        let fragment_id =
            jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr.rest_address());
        utxo = jcli_wrapper::assert_rest_utxo_get(
            &jormungandr.rest_address(),
            &fragment_id.to_hex(),
            0,
        );

        assert_funds_transferred_to(&receiver.address, &utxo);
        jormungandr.assert_no_errors_in_log();
        std::mem::swap(&mut sender, &mut receiver);
    }

    jcli_wrapper::assert_all_transaction_log_shows_in_block(&jormungandr.rest_address());
}

fn assert_funds_transferred_to(address: &str, utxo: &UTxOInfo) {
    assert_eq!(
        &utxo.address().to_string(),
        &address,
        "funds were transfer on wrong account (or didn't at all). Utxo: {:?}, receiver address: {:?}",utxo,address
    );
}

#[test]
pub fn test_blocks_are_being_created_for_more_than_15_minutes() {
    let mut sender = startup::create_new_utxo_address();
    let mut receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_block0_consensus("bft")
        .with_kes_update_speed(43200)
        .with_slots_per_epoch(5)
        .with_slot_duration(2)
        .with_epoch_stability_depth(10)
        .build();

    let mut utxo = config.block0_utxo_for_address(&sender);
    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let now = SystemTime::now();
    loop {
        let new_transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address.clone(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();

        let fragment_id = jcli_wrapper::assert_transaction_in_block(
            &new_transaction,
            &jormungandr.rest_address(),
        );
        utxo = jcli_wrapper::assert_rest_utxo_get(
            &jormungandr.rest_address(),
            &fragment_id.to_hex(),
            0,
        );
        assert_funds_transferred_to(&receiver.address, &utxo);

        // 900 s = 15 minutes
        if now.elapsed().unwrap().as_secs() > 900 {
            break;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}
