use crate::common::{
    configuration::genesis_model::Fund, jcli_wrapper,
    jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper, jormungandr::starter, startup,
};

#[test]
pub fn node_restart() {
    let sender = startup::create_new_utxo_address();
    let account_receiver = startup::create_new_account_address();
    let utxo_receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address().clone();
    let mut jormungandr = startup::start_jormungandr_node_as_leader(&mut config);

    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

    let transaction_message = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash)
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&account_receiver.address, &50.into())
        .assert_add_output(&utxo_receiver.address, &50.into())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);

    let expected_settings = jcli_wrapper::assert_get_rest_settings(&jormungandr_rest_address);
    let expected_utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let expected_account_state = jcli_wrapper::assert_rest_account_get_stats(
        &account_receiver.address,
        &jormungandr_rest_address,
    );

    let _jormungandr_after_restart = starter::restart_jormungandr_node_as_leader(&mut jormungandr);

    let actual_settings = jcli_wrapper::assert_get_rest_settings(&jormungandr_rest_address);
    let actual_utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let actual_account_state = jcli_wrapper::assert_rest_account_get_stats(
        &account_receiver.address,
        &jormungandr_rest_address,
    );

    assert_eq!(
        actual_settings, expected_settings,
        "Different setting after restart {:?} vs {:?}",
        actual_settings, expected_settings
    );
    assert_eq!(
        actual_utxos, expected_utxos,
        "Different utxos after restart {:?} vs {:?}",
        actual_utxos, expected_utxos
    );
    assert_eq!(
        actual_account_state, expected_account_state,
        "Different account state after restart {:?} vs {:?}",
        actual_account_state, expected_account_state
    );
}
