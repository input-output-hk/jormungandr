use crate::common::configuration::genesis_model::{Fund, LinearFees};
use crate::common::jcli_wrapper;
use crate::common::jcli_wrapper::jcli_transaction_wrapper::JCLITransactionWrapper;
use crate::common::startup;
use jormungandr_lib::{crypto::hash::Hash, interfaces::Value};

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}

#[test]
pub fn test_utxo_transaction_with_more_than_one_witness_per_input_is_rejected() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path);

    let mut transaction_wrapper = JCLITransactionWrapper::new_transaction(&block0_hash);
    transaction_wrapper
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &utxo.associated_fund())
        .assert_finalize();

    let witness1 = transaction_wrapper.create_witness_default("utxo");
    let witness2 = transaction_wrapper.create_witness_default("utxo");

    transaction_wrapper
        .assert_make_witness(&witness1)
        .assert_add_witness(&witness1)
        .assert_make_witness(&witness2)
        .assert_add_witness_fail(
            &witness2,
            "too many witnesses in transaction to add another: 1, maximum is 1",
        );
}

#[test]
pub fn test_two_correct_utxo_to_utxo_transactions_are_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let middle_man = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);

    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path);
    let transaction_builder = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &middle_man,
        &utxo.associated_fund(),
        &sender,
        &block0_hash,
    );

    let transaction_message = transaction_builder.assert_transaction_to_message();
    let first_transaction_id = transaction_builder.get_transaction_id();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);

    let transaction_message = JCLITransactionWrapper::build_transaction(
        &first_transaction_id.parse().unwrap(),
        0,
        &100.into(),
        &receiver,
        &100.into(),
        &middle_man,
        &block0_hash,
    )
    .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_correct_utxo_transaction_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &utxo.associated_fund())
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo");

    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_correct_utxo_transaction_replaces_old_utxo_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &utxo.associated_fund())
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo")
        .assert_transaction_to_message();
    let transaction_id = transaction_builder.get_transaction_id();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);

    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    assert_eq!(utxos.len(), 1);

    let utxo = &utxos[0];
    assert_eq!(
        utxo.address().to_string(),
        receiver.address,
        "after successful transaction out_addr for utxo should be equal to receiver address"
    );
    assert_eq!(
        *utxo.associated_fund(),
        Value::from(100),
        "out value should be equal to output of first transaction"
    );
    assert_eq!(
        utxo.index_in_transaction(),
        0,
        "since only one transaction was made, idx should be equal to 1"
    );
    assert_eq!(
        utxo.transaction_id().to_hex(),
        transaction_id,
        "transaction hash should be equal to new transaction"
    );
}

#[test]
pub fn test_account_is_created_if_transaction_out_is_account() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);

    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo");

    let transaction_message = transaction_builder.assert_transaction_to_message();

    // assert account received funds
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
    let account_state =
        jcli_wrapper::assert_rest_account_get_stats(&receiver.address, &jormungandr_rest_address);
    assert_eq!(
        account_state.value().to_string(),
        transfer_amount.to_string(),
        "Account did not receive correct amount of funds"
    );

    // assert utxo are empty now
    let utxos = jcli_wrapper::assert_rest_utxo_get(&jormungandr_rest_address);
    assert_eq!(utxos.len(), 0);
}

#[test]
pub fn test_transaction_from_delegation_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo")
        .assert_transaction_to_message();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_delegation_to_account_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo")
        .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_delegation_to_utxo_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_utxo_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo")
        .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_utxo_to_account_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_account_address();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &utxo.associated_fund())
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo")
        .assert_transaction_to_message();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_account_to_account_is_accepted_by_node() {
    let sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    let transaction_message = transaction_builder
        .assert_add_account(&sender.address, &transfer_amount)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "account")
        .assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_account_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_account_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    transaction_builder
        .assert_add_account(&sender.address, &transfer_amount)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "account");

    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_transaction_from_utxo_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address, &transfer_amount)
        .assert_finalize()
        .seal_with_witness_default(&sender.private_key, "utxo");
    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_input_with_smaller_value_than_initial_utxo_is_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let transaction_builder = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &99.into(),
        &receiver,
        &99.into(),
        &sender,
        &block0_hash,
    );
    let transaction_message = transaction_builder.assert_transaction_to_message();
    jcli_wrapper::assert_transaction_rejected(
        &transaction_message,
        &jormungandr_rest_address,
        "The UTxO value (99) in the transaction does not match the actually state value: 100",
    );
}

#[test]
pub fn test_transaction_with_non_existing_id_should_be_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();
    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path);
    let transaction_builder = JCLITransactionWrapper::build_transaction(
        &FAKE_INPUT_TRANSACTION_ID,
        0,
        &100.into(),
        &receiver,
        &50.into(),
        &sender,
        &block0_hash,
    );
    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_rejected(
        &transaction_message,
        &jormungandr_rest_address,
        "Invalid UTxO",
    );
}

#[test]
pub fn test_transaction_with_input_address_equal_to_output_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let transaction_builder = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &sender,
        &utxo.associated_fund(),
        &sender,
        &config.genesis_block_hash,
    );
    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);
}

#[test]
pub fn test_input_with_no_spending_utxo_is_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let transaction_builder = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &100.into(),
        &receiver,
        &50.into(),
        &sender,
        &config.genesis_block_hash,
    );
    let transaction_message = transaction_builder.assert_transaction_to_message();

    jcli_wrapper::assert_transaction_rejected(
        &transaction_message,
        &jormungandr_rest_address,
        "Inputs, outputs and fees are not balanced, transaction with 100 input and 50 output",
    );
}

#[test]
pub fn test_transaction_with_non_zero_linear_fees() {
    let sender = startup::create_new_utxo_address();
    let reciever = startup::create_new_utxo_address();
    let mut config = startup::ConfigurationBuilder::new()
        .with_funds(vec![Fund {
            address: sender.address.clone(),
            value: 100.into(),
        }])
        .with_linear_fees(LinearFees {
            constant: 10,
            coefficient: 1,
            certificate: 0,
        })
        .build();

    let jormungandr_rest_address = config.get_node_address();
    let _jormungandr = startup::start_jormungandr_node_as_leader(&mut config);
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    let mut transaction_builder =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash);
    transaction_builder
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&reciever.address, &50.into())
        .assert_finalize_with_fee(
            &sender.address,
            &LinearFees {
                constant: 10,
                coefficient: 1,
                certificate: 0,
            },
        )
        .seal_with_witness_default(&sender.private_key, "utxo");
    let transaction_message = transaction_builder.assert_transaction_to_message();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr_rest_address);

    let utxo = startup::get_utxo_for_address(&reciever, &jormungandr_rest_address);
    assert_eq!(
        utxo.associated_fund(),
        &Value::from(50),
        "Wrong funds amount on receiver account"
    );
    let utxo = startup::get_utxo_for_address(&sender, &jormungandr_rest_address);
    assert_eq!(
        utxo.associated_fund(),
        &Value::from(37),
        "Wrong remaining funds amount on sender account"
    );
}
