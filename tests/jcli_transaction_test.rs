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
pub fn test_unbalanced_output_utxo_transation_is_rejected() {
    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new();
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);

    let jcli_transaction_wrapper = JCLITransactionWrapper::new();
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);

    let first_utxo = &utxos[0];
    let second_utxo = &utxos[1];

    jcli_transaction_wrapper.assert_new_transaction();
    jcli_transaction_wrapper.assert_add_input(
        &first_utxo.in_txid,
        &first_utxo.in_idx,
        &first_utxo.out_value,
    );

    jcli_transaction_wrapper.assert_add_output(
        &second_utxo.out_addr,
        &(second_utxo.out_value + first_utxo.out_value),
    );

    jcli_transaction_wrapper.assert_finalize_fail("not enough input for making transaction");
}

#[test]
pub fn test_utxo_transation_with_more_than_one_witness_per_input_is_rejected() {
    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new();
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);
    let block0_hash = startup::get_genesis_block_hash(&genesis_model);

    let jcli_transaction_wrapper = JCLITransactionWrapper::new();
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);

    let first_utxo = &utxos[0];
    let second_utxo = &utxos[1];

    jcli_transaction_wrapper.assert_new_transaction();
    jcli_transaction_wrapper.assert_add_input(
        &first_utxo.in_txid,
        &first_utxo.in_idx,
        &first_utxo.out_value,
    );

    jcli_transaction_wrapper.assert_add_output(&second_utxo.out_addr, &first_utxo.out_value);

    jcli_transaction_wrapper.assert_finalize();

    let witness_key = jcli_wrapper::assert_key_generate_default();
    jcli_transaction_wrapper.save_witness_key(&witness_key);
    let transaction_id = jcli_transaction_wrapper.get_transaction_id();

    jcli_transaction_wrapper.assert_make_witness(&block0_hash, &transaction_id, "utxo", &0);
    jcli_transaction_wrapper.assert_add_witness();

    jcli_transaction_wrapper.assert_add_witness_fail("cannot add anymore witnesses");
}

#[test]
pub fn test_correct_utxo_transaction_is_accepted_by_node() {
    let jcli_transaction_wrapper = JCLITransactionWrapper::new();
    let sender_priv = jcli_wrapper::assert_key_generate_default();
    let sender_pub = jcli_wrapper::assert_key_to_public_default(&sender_priv);
    let sender_addr = jcli_wrapper::assert_address_single_default(&sender_pub);
    let recv_priv = jcli_wrapper::assert_key_generate_default();
    let recv_pub = jcli_wrapper::assert_key_to_public_default(&recv_priv);
    let recv_addr = jcli_wrapper::assert_address_single_default(&recv_pub);

    let node_config = NodeConfig::new();
    let genesis_model = GenesisYaml::new_with_funds(vec![Fund {
        address: sender_addr,
        value: 100,
    }]);
    let jormungandr_rest_address = node_config.get_node_address();
    let _jormungandr =
        startup::start_jormungandr_node_with_genesis_conf(&genesis_model, &node_config);
    let block0_hash = startup::get_genesis_block_hash(&genesis_model);

    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);

    let utxo = &utxos[0];

    jcli_transaction_wrapper.assert_new_transaction();
    jcli_transaction_wrapper.assert_add_input(&utxo.in_txid, &utxo.in_idx, &utxo.out_value);

    jcli_transaction_wrapper.assert_add_output(&recv_addr, &utxo.out_value);

    jcli_transaction_wrapper.assert_finalize();
    jcli_transaction_wrapper.save_witness_key(&sender_priv);
    let transaction_id = jcli_transaction_wrapper.get_transaction_id();

    jcli_transaction_wrapper.assert_make_witness(&block0_hash, &transaction_id, "utxo", &0);
    jcli_transaction_wrapper.assert_add_witness();

    jcli_transaction_wrapper.assert_seal();

    let transaction_message = jcli_transaction_wrapper.assert_transaction_to_message();

    jcli_wrapper::assert_post_transaction(&transaction_message, &jormungandr_rest_address);

    let node_stats = jcli_wrapper::assert_rest_stats(&jormungandr_rest_address);

    assert_eq!("1", node_stats.get("txRecvCnt").unwrap());
}
