#![cfg(feature = "integration-test")]

extern crate assert_cmd;
extern crate galvanic_test;
extern crate mktemp;

mod common;

use common::configuration::genesis_model::{Fund, GenesisYaml};
use common::configuration::node_config_model::NodeConfig;
use common::jcli_wrapper;
use common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use common::startup;

#[test]
#[cfg(feature = "integration-test")]
pub fn test_unbalanced_output_utxo_transation_is_rejected() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(vec![Fund {
        address: sender.address,
        value: 100,
    }]);
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);

    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let first_utxo = &utxos[0];

    JCLITransactionWrapper::new()
        .assert_new_transaction()
        .assert_add_input(
            &first_utxo.in_txid,
            &first_utxo.in_idx,
            &first_utxo.out_value,
        )
        .assert_add_output(&reciever.address, &(first_utxo.out_value + 100))
        .assert_finalize_fail("not enough input for making transaction");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_utxo_transation_with_more_than_one_witness_per_input_is_rejected() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(vec![Fund {
        address: sender.address,
        value: 100,
    }]);
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let first_utxo = &utxos[0];

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction();

    let witness1 = transaction_wrapper.create_witness_default("utxo", &jormungandr_rest_address);
    let witness2 = transaction_wrapper.create_witness_default("utxo", &jormungandr_rest_address);

    transaction_wrapper
        .assert_add_input(
            &first_utxo.in_txid,
            &first_utxo.in_idx,
            &first_utxo.out_value,
        )
        .assert_add_output(&reciever.address, &first_utxo.out_value)
        .assert_finalize()
        .assert_make_witness(&witness1)
        .assert_add_witness(&witness1)
        .assert_make_witness(&witness2)
        .assert_add_witness_fail(&witness2, "cannot add anymore witnesses");
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_correct_utxo_transaction_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(vec![Fund {
        address: sender.address,
        value: 100,
    }]);

    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let utxo = &utxos[0];

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction();
    transaction_wrapper
        .assert_add_input(&utxo.in_txid, &utxo.in_idx, &utxo.out_value)
        .assert_add_output(&reciever.address, &utxo.out_value)
        .assert_finalize();

    let witness = transaction_wrapper.create_witness_from_key(
        &sender.private_key,
        "utxo",
        &jormungandr_rest_address,
    );

    let transaction_message = transaction_wrapper
        .seal_with_witness(&witness)
        .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_post_accepted(&transaction_message, &jormungandr_rest_address);
}

#[test]
#[cfg(feature = "integration-test")]
pub fn test_transaction_from_utxo_to_account_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();

    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(vec![Fund {
        address: sender.address,
        value: 100,
    }]);

    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    let utxo = &utxos[0];

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction();
    transaction_wrapper
        .assert_add_input(&utxo.in_txid, &utxo.in_idx, &utxo.out_value)
        .assert_add_output(&reciever.address, &utxo.out_value)
        .assert_finalize();

    let witness = transaction_wrapper.create_witness_from_key(
        &sender.private_key,
        "account",
        &jormungandr_rest_address,
    );

    let transaction_message = transaction_wrapper.assert_make_witness_fail(
        &witness,
        "index out of bounds: the len is 0 but the index is 0",
    );

    /*
    Assertion is changed due to issue: ##325
    After fix please revert it to:

    let transaction_message = transaction_wrapper
        .seal_with_witness(&witness)
        .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_post_failed(&transaction_message, &jormungandr_rest_address);
    */
}
