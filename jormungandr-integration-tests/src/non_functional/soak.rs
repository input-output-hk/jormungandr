#![cfg(feature = "soak-test")]

use crate::common::configuration::genesis_model::Fund;
use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::process_utils;
use crate::common::startup;

use std::time::SystemTime;

#[test]
pub fn test_100_transaction_is_processed() {
    let mut sender = startup::create_new_utxo_address();
    let mut receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let jormungandr = startup::start_jormungandr_node_as_leader(&mut config);

    for _i in 0..100 {
        let utxo = startup::get_utxos_for_address(&sender, &jormungandr_rest_address);

        let transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address.clone(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();

        jcli_wrapper::assert_transaction_in_block(&transaction, &jormungandr_rest_address);

        assert_funds_transferred_to(&receiver.address, &jormungandr_rest_address);
        jormungandr.assert_no_errors_in_log();
        std::mem::swap(&mut sender, &mut receiver);
    }

    jcli_wrapper::assert_all_transaction_log_shows_in_block(&jormungandr_rest_address);
}

fn assert_funds_transferred_to(address: &str, host: &str) {
    let utxos = jcli_wrapper::assert_rest_utxos_get(&host);
    assert_eq!(utxos.len(), 1, "Only one utxo expected");
    assert_eq!(
        &utxos[0].address().to_string(),
        &address,
        "funds were transfer on wrong account (or didn't at all). Utxos: {:?}, receiver address: {:?}",utxos,address
    );
}

#[test]
pub fn test_blocks_are_being_created_for_more_than_15_minutes() {
    let mut sender = startup::create_new_utxo_address();
    let mut receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .with_consensus_genesis_praos_active_slot_coeff("0.1")
        .with_block0_consensus("bft")
        .with_bft_slots_ratio("0".to_owned())
        .with_kes_update_speed(43200)
        .with_slots_per_epoch(5)
        .with_slot_duration(2)
        .with_epoch_stability_depth(10)
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let now = SystemTime::now();
    loop {
        let utxo = startup::get_utxos_for_address(&sender, &jormungandr_rest_address);

        let new_transaction = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address.clone(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();

        super::send_transaction_and_ensure_block_was_produced(&vec![new_transaction], &jormungandr);
        assert_funds_transferred_to(&receiver.address, &jormungandr_rest_address);

        // 900 s = 15 minutes
        if now.elapsed().unwrap().as_secs() > 900 {
            break;
        }

        std::mem::swap(&mut sender, &mut receiver);
    }
}
