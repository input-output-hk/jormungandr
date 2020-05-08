use crate::common::{
    jcli_wrapper::{self, jcli_transaction_wrapper::JCLITransactionWrapper},
    jormungandr::{ConfigurationBuilder, Starter},
    startup,
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{InitialUTxO, UTxOInfo},
};

use chain_impl_mockchain::fee::LinearFee;

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
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let _ = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);

    let mut transaction_wrapper =
        JCLITransactionWrapper::new_transaction(&config.genesis_block_hash());
    let transaction_wrapper = transaction_wrapper
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &utxo.associated_fund())
        .assert_finalize();

    let witness1 = transaction_wrapper.create_witness_default("utxo", None);
    let witness2 = transaction_wrapper.create_witness_default("utxo", None);

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

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();

    let utxo = config.block0_utxo_for_address(&sender);
    let block0_hash = jcli_wrapper::assert_genesis_hash(config.genesis_block_path());
    let first_transaction = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &middle_man,
        &utxo.associated_fund(),
        &sender,
        &block0_hash,
    );

    let first_transaction_id =
        jcli_wrapper::assert_transaction_in_block(&first_transaction, &jormungandr);

    let second_transaction = JCLITransactionWrapper::build_transaction(
        &first_transaction_id,
        0,
        &100.into(),
        &receiver,
        &100.into(),
        &middle_man,
        &block0_hash,
    );
    jcli_wrapper::assert_transaction_in_block(&second_transaction, &jormungandr);
}

#[test]
pub fn test_correct_utxo_transaction_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    println!("Sender: {:?}", sender);
    println!("Receiver: {:?}", sender);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config).start().unwrap();

    let utxo = jormungandr.config.block0_utxo_for_address(&sender);
    let transaction_message =
        JCLITransactionWrapper::new_transaction(&jormungandr.config.genesis_block_hash())
            .assert_add_input_from_utxo(&utxo)
            .assert_add_output(&receiver.address().to_string(), &utxo.associated_fund())
            .assert_finalize()
            .seal_with_witness_for_address(&sender)
            .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_correct_utxo_transaction_replaces_old_utxo_by_node() {
    const TX_VALUE: u64 = 100;

    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: TX_VALUE.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let rest_addr = jormungandr.rest_address();
    let utxo = config.block0_utxo_for_address(&sender);

    let mut tx = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash());
    let tx_message = tx
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &utxo.associated_fund())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();
    let new_utxo = UTxOInfo::new(tx.get_fragment_id(), 0, receiver.address(), TX_VALUE.into());

    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&rest_addr, &utxo);
    jcli_wrapper::assert_rest_utxo_get_by_utxo_not_found(&rest_addr, &new_utxo);

    jcli_wrapper::assert_transaction_in_block(&tx_message, &jormungandr);

    jcli_wrapper::assert_rest_utxo_get_by_utxo_not_found(&rest_addr, &utxo);
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&rest_addr, &new_utxo);
}

#[test]
pub fn test_account_is_created_if_transaction_out_is_account() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);

    let transaction_message = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash())
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    // assert utxo does contains TX
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&jormungandr.rest_address(), &utxo);

    // assert account received funds
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);

    let account_state = jcli_wrapper::assert_rest_account_get_stats(
        &receiver.address().to_string(),
        &jormungandr.rest_address(),
    );
    assert_eq!(
        account_state.value().to_string(),
        transfer_amount.to_string(),
        "Account did not receive correct amount of funds"
    );

    // assert utxo does not contain TX anymore
    jcli_wrapper::assert_rest_utxo_get_not_found(
        &jormungandr.rest_address(),
        &utxo.transaction_id().to_string(),
        utxo.index_in_transaction(),
    );
}

#[test]
pub fn test_transaction_from_delegation_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_delegation_to_account_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_delegation_to_utxo_is_accepted_by_node() {
    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_utxo_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_utxo_to_account_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_account_address();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &utxo.associated_fund())
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_account_to_account_is_accepted_by_node() {
    let sender = startup::create_new_account_address();
    let receiver = startup::create_new_account_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_account(&sender.address().to_string(), &transfer_amount)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_account_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_account_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_account(&sender.address().to_string(), &transfer_amount)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();
    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_transaction_from_utxo_to_delegation_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::new(&config.genesis_block_hash())
        .assert_new_transaction()
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &transfer_amount)
        .assert_finalize()
        .seal_with_witness_for_address(&sender)
        .assert_to_message();

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_input_with_smaller_value_than_initial_utxo_is_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path());
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &99.into(),
        &receiver,
        &99.into(),
        &sender,
        &block0_hash,
    );
    jcli_wrapper::assert_transaction_rejected(
        &transaction_message,
        &jormungandr,
        "The UTxO value (99) in the transaction does not match the actually state value: 100",
    );
}

#[test]
pub fn test_transaction_with_non_existing_id_should_be_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();
    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let block0_hash = jcli_wrapper::assert_genesis_hash(&config.genesis_block_path());
    let transaction_message = JCLITransactionWrapper::build_transaction(
        &FAKE_INPUT_TRANSACTION_ID,
        0,
        &100.into(),
        &receiver,
        &100.into(),
        &sender,
        &block0_hash,
    );
    jcli_wrapper::assert_transaction_rejected(&transaction_message, &jormungandr, "Invalid UTxO");
}

#[test]
pub fn test_transaction_with_input_address_equal_to_output_is_accepted_by_node() {
    let sender = startup::create_new_utxo_address();
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &utxo.associated_fund(),
        &sender,
        &utxo.associated_fund(),
        &sender,
        &config.genesis_block_hash(),
    );

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);
}

#[test]
pub fn test_input_with_no_spending_utxo_is_rejected_by_node() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let transaction_message = JCLITransactionWrapper::build_transaction_from_utxo(
        &utxo,
        &100.into(),
        &receiver,
        &50.into(),
        &sender,
        &config.genesis_block_hash(),
    );

    jcli_wrapper::assert_transaction_rejected(
        &transaction_message,
        &jormungandr,
        "Failed to validate transaction balance: transaction value not balanced, has inputs sum 100 and outputs sum 50",
    );
}

#[test]
pub fn test_transaction_with_non_zero_linear_fees() {
    let sender = startup::create_new_utxo_address();
    let receiver = startup::create_new_utxo_address();
    let fee = LinearFee::new(10, 1, 0);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .with_linear_fees(fee.clone())
        .build();

    let jormungandr = Starter::new().config(config.clone()).start().unwrap();
    let utxo = config.block0_utxo_for_address(&sender);
    let mut tx = JCLITransactionWrapper::new_transaction(&config.genesis_block_hash());
    let transaction_message = tx
        .assert_add_input_from_utxo(&utxo)
        .assert_add_output(&receiver.address().to_string(), &50.into())
        .assert_finalize_with_fee(&sender.address().to_string(), &fee)
        .seal_with_witness_for_address(&sender)
        .assert_to_message();
    let tx_id = tx.get_fragment_id();
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(&jormungandr.rest_address(), &utxo);

    jcli_wrapper::assert_transaction_in_block(&transaction_message, &jormungandr);

    jcli_wrapper::assert_rest_utxo_get_by_utxo_not_found(&jormungandr.rest_address(), &utxo);
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(
        &jormungandr.rest_address(),
        &UTxOInfo::new(tx_id, 0, receiver.address(), 50.into()),
    );
    jcli_wrapper::assert_rest_utxo_get_returns_same_utxo(
        &jormungandr.rest_address(),
        &UTxOInfo::new(tx_id, 1, sender.address(), 37.into()),
    );
}
