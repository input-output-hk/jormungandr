#![cfg(feature = "soak-test")]

use crate::common::configuration::genesis_model::Fund;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;

use crate::common::jcli_wrapper;
use crate::common::process_utils;

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
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);

    for _i in 0..100 {
        let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

        let mut transaction_builder =
            JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
        transaction_builder
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address.clone(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_default(&sender.private_key.clone(), "utxo");

        assert_transaction_in_block(transaction_builder, &jormungandr_rest_address);
        assert_funds_transferred_to(&receiver.address, &jormungandr_rest_address);

        std::mem::swap(&mut sender, &mut receiver);
    }

    process_utils::sleep(1);
    let message_logs = jcli_wrapper::assert_rest_message_logs(&jormungandr_rest_address);
    message_logs
        .iter()
        .for_each(|el| assert!(el.is_in_a_block()));
}

fn assert_transaction_in_block(transaction_builder: JCLITransactionWrapper, host: &str) {
    let transaction_message = transaction_builder.assert_transaction_to_message();
    let transaction_id = transaction_builder.get_transaction_id();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &transaction_id, &host);
}

fn assert_funds_transferred_to(address: &str, host: &str) {
    let utxos = jcli_wrapper::assert_rest_utxo_get(&host);
    assert_eq!(utxos.len(), 1, "Only one utxo expected");
    assert_eq!(
        &utxos[0].address().to_string(),
        &address,
        "funds were transfer on wrong account (or didn't at all)"
    );
}
