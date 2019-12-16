#![cfg(feature = "soak-test")]

use crate::common::{
    configuration::genesis_model::Fund,
    data::address::Account,
    jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper,
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};

use std::iter;

#[test]
pub fn test_100_transaction_is_processed_in_10_packs() {
    let receivers: Vec<Account> = iter::from_fn(|| Some(startup::create_new_account_address()))
        .take(10)
        .collect();
    send_100_transaction_in_10_packs_for_recievers(10, receivers)
}

#[test]
#[ignore] //The Node stops creating blocks (BFT and Genesis) #591
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
    let sender = startup::create_new_account_address();

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
            JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
                .assert_add_account(&sender.address.clone(), &output_value.into())
                .assert_add_output(&receiver.address.clone(), &output_value.into())
                .assert_finalize()
                .seal_with_witness_for_address(&sender)
                .assert_to_message()
        })
        .collect();

    for _ in 0..iterations_count {
        super::send_transaction_and_ensure_block_was_produced(&transation_messages, &jormungandr);
    }
}
