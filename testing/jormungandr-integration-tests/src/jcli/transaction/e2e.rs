use crate::startup;
use assert_fs::TempDir;
use chain_impl_mockchain::fee::LinearFee;
use jormungandr_automation::{
    jcli::{JCli, WitnessType},
    jormungandr::{ConfigurationBuilder, Starter},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, InitialUTxO, UTxOInfo},
};

lazy_static! {
    static ref FAKE_INPUT_TRANSACTION_ID: Hash = {
        "19c9852ca0a68f15d0f7de5d1a26acd67a3a3251640c6066bdb91d22e2000193"
            .parse()
            .unwrap()
    };
}

#[test]
pub fn test_utxo_transaction_with_more_than_one_witness_per_input_is_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let _jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    let mut transaction_builder = jcli.transaction_builder(block0_hash);
    transaction_builder
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), *utxo.associated_fund())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize();

    let witness1 = transaction_builder.create_witness_default(WitnessType::UTxO, None);
    let witness2 = transaction_builder.create_witness_default(WitnessType::UTxO, None);

    transaction_builder
        .make_witness(&witness1)
        .add_witness(&witness1)
        .make_witness(&witness2)
        .add_witness_expect_fail(
            &witness2,
            "too many witnesses in transaction to add another: 1, maximum is 1",
        );
}

#[test]
pub fn test_two_correct_utxo_to_utxo_transactions_are_accepted_by_node() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let middle_man = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();

    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = jcli.genesis().hash(config.genesis_block_path());
    let first_transaction = jcli
        .transaction_builder(block0_hash)
        .build_transaction_from_utxo(
            &utxo,
            *utxo.associated_fund(),
            sender.witness_data(),
            *utxo.associated_fund(),
            &middle_man.address(),
            BlockDate::new(1, 0),
        );

    let first_transaction_id = jcli
        .fragment_sender(&jormungandr)
        .send(&first_transaction)
        .assert_in_block();

    let second_transaction = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input(&first_transaction_id.into(), 0, &100.to_string())
        .add_output(&receiver.address().to_string(), 100.into())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(middle_man.witness_data())
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&second_transaction)
        .assert_in_block();
}

#[test]
pub fn test_correct_utxo_transaction_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    println!("Sender: {:?}", sender);
    println!("Receiver: {:?}", sender);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();

    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), *utxo.associated_fund())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_correct_utxo_transaction_replaces_old_utxo_by_node() {
    let jcli: JCli = Default::default();
    const TX_VALUE: u64 = 100;

    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: TX_VALUE.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let rest_addr = jormungandr.rest_uri();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    let mut tx = jcli.transaction_builder(block0_hash);
    let tx_message = tx
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), *utxo.associated_fund())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();
    let new_utxo = UTxOInfo::new(tx.fragment_id(), 0, receiver.address(), TX_VALUE.into());

    jcli.rest().v0().utxo().assert_contains(&utxo, &rest_addr);
    jcli.rest()
        .v0()
        .utxo()
        .expect_item_not_found(&new_utxo, &rest_addr);

    jcli.fragment_sender(&jormungandr)
        .send(&tx_message)
        .assert_in_block();

    jcli.rest()
        .v0()
        .utxo()
        .expect_item_not_found(&utxo, &rest_addr);
    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&new_utxo, &rest_addr);
}

#[test]
pub fn test_account_is_created_if_transaction_out_is_account() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::default();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    // assert utxo does contains TX
    jcli.rest()
        .v0()
        .utxo()
        .assert_contains(&utxo, &jormungandr.rest_uri());

    // assert account received funds
    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();

    let account_state = jcli
        .rest()
        .v0()
        .account_stats(receiver.address().to_string(), jormungandr.rest_uri());
    assert_eq!(
        account_state.value().to_string(),
        transfer_amount.to_string(),
        "Account did not receive correct amount of funds"
    );

    // assert utxo does not contain TX anymore
    jcli.rest().v0().utxo().expect_not_found(
        utxo.transaction_id().to_string(),
        utxo.index_in_transaction(),
        jormungandr.rest_uri(),
    );
}

#[test]
pub fn test_transaction_from_delegation_to_delegation_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = startup::create_new_delegation_address();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_delegation_to_account_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = startup::create_new_delegation_address();
    let receiver = thor::Wallet::default();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_delegation_to_utxo_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = startup::create_new_delegation_address();
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_utxo_to_account_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::default();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), *utxo.associated_fund())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_account_to_account_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::default();
    let receiver = thor::Wallet::default();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_account(&sender.address().to_string(), &transfer_amount)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_account_to_delegation_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::default();
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_account(&sender.address().to_string(), &transfer_amount)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();
    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_transaction_from_utxo_to_delegation_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = startup::create_new_delegation_address();
    let transfer_amount = 100.into();

    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: transfer_amount,
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), transfer_amount)
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_input_with_smaller_value_than_initial_utxo_is_rejected_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let block0_hash = jcli.genesis().hash(config.genesis_block_path());
    let utxo = config.block0_utxo_for_address(&sender.address());
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .build_transaction_from_utxo(
            &utxo,
            99.into(),
            sender.witness_data(),
            99.into(),
            &receiver.address(),
            BlockDate::new(1, 0),
        );

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_rejected(
            "The UTxO value (99) in the transaction does not match the actually state value: 100",
        );
}

#[test]
pub fn test_transaction_with_non_existing_id_should_be_rejected_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();
    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);
    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let block0_hash = jcli.genesis().hash(config.genesis_block_path());
    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input(&FAKE_INPUT_TRANSACTION_ID, 0, &100.to_string())
        .add_output(&receiver.address().to_string(), 100.into())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .seal_with_witness_data(sender.witness_data())
        .to_message();

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_rejected("Invalid UTxO");
}

#[test]
pub fn test_transaction_with_input_address_equal_to_output_is_accepted_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let transaction_message = jcli.transaction_builder(block).build_transaction_from_utxo(
        &utxo,
        *utxo.associated_fund(),
        sender.witness_data(),
        *utxo.associated_fund(),
        &sender.address(),
        BlockDate::new(1, 0),
    );

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();
}

#[test]
pub fn test_input_with_no_spending_utxo_is_rejected_by_node() {
    let temp_dir = TempDir::new().unwrap();
    let jcli: JCli = Default::default();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();

    let transaction_message = jcli
        .transaction_builder(block0_hash)
        .build_transaction_from_utxo(
            &utxo,
            100.into(),
            sender.witness_data(),
            50.into(),
            &receiver.address(),
            BlockDate::new(1, 0),
        );

    jcli.fragment_sender(&jormungandr).send(&transaction_message).assert_rejected(
        "Failed to validate transaction balance: transaction value not balanced, has inputs sum 100 and outputs sum 50"
    );
}

#[test]
pub fn test_transaction_with_non_zero_linear_fees() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let fee = LinearFee::new(10, 1, 0);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .with_linear_fees(fee.clone())
        .build(&temp_dir);

    let jormungandr = Starter::new()
        .temp_dir(temp_dir)
        .config(config.clone())
        .start()
        .unwrap();
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let mut tx = jcli.transaction_builder(block0_hash);
    let transaction_message = tx
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), 50.into())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize_with_fee(&sender.address().to_string(), &fee)
        .seal_with_witness_data(sender.witness_data())
        .to_message();
    let tx_id = tx.fragment_id();
    let rest_uri = jormungandr.rest_uri();
    jcli.rest().v0().utxo().assert_contains(&utxo, &rest_uri);

    jcli.fragment_sender(&jormungandr)
        .send(&transaction_message)
        .assert_in_block();

    jcli.rest()
        .v0()
        .utxo()
        .expect_item_not_found(&utxo, &rest_uri);
    jcli.rest().v0().utxo().assert_contains(
        &UTxOInfo::new(tx_id, 0, receiver.address(), 50.into()),
        &rest_uri,
    );
    jcli.rest().v0().utxo().assert_contains(
        &UTxOInfo::new(tx_id, 1, sender.address(), 37.into()),
        &rest_uri,
    );
}

#[test]
fn test_cannot_create_transaction_without_expiration() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    jcli.transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), 50.into())
        .finalize_expect_fail("cannot finalize the payload without a validity end date set");
}

#[test]
fn test_different_transaction_expiry_yields_different_id() {
    let jcli: JCli = Default::default();
    let temp_dir = TempDir::new().unwrap();

    let sender = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let receiver = thor::Wallet::new_utxo(&mut rand::rngs::OsRng);
    let config = ConfigurationBuilder::new()
        .with_funds(vec![InitialUTxO {
            address: sender.address(),
            value: 100.into(),
        }])
        .build(&temp_dir);
    let utxo = config.block0_utxo_for_address(&sender.address());
    let block0_hash = Hash::from_hex(config.genesis_block_hash()).unwrap();
    let first_id = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), 50.into())
        .set_expiry_date(BlockDate::new(1, 0))
        .finalize()
        .transaction_id();
    let second_id = jcli
        .transaction_builder(block0_hash)
        .new_transaction()
        .add_input_from_utxo(&utxo)
        .add_output(&receiver.address().to_string(), 50.into())
        .set_expiry_date(BlockDate::new(2, 0))
        .finalize()
        .transaction_id();

    assert_ne!(first_id, second_id);
}
