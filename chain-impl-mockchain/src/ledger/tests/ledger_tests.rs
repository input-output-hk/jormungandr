#![cfg(test)]

use crate::testing::address::AddressData;
use crate::testing::ledger;
use crate::testing::ledger::ConfigBuilder;
use crate::testing::tx_builder::TransactionBuilder;
use chain_addr::Discrimination;
use crate::ledger::Error::{NotEnoughSignatures, TransactionHasTooManyOutputs};
use crate::transaction::*;
use crate::value::*;

macro_rules! assert_err {
    ($left: expr, $right: expr) => {
        match &($left) {
            left_val => match &($right) {
                Err(e) => {
                    if !(e == left_val) {
                        panic!(
                            "assertion failed: error mismatch \
                             (left: `{:?}, right: `{:?}`)",
                            *left_val, *e
                        )
                    }
                }
                Ok(_) => panic!(
                    "assertion failed: expected error {:?} but got success",
                    *left_val
                ),
            },
        }
    };
}

#[test]
pub fn utxo_no_enough_signatures() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);

    let (message, utxos) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));
    let (_, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from_address(receiver.address.clone(), Value(1)))
        .authenticate()
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        NotEnoughSignatures {
            actual: 0,
            expected: 1
        },
        ledger.apply_transaction(&signed_tx, &fees)
    )
}

#[test]
pub fn utxo_to_utxo_correct_transaction() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);

    let (message, utxos) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from_address(receiver.address.clone(), Value(42000)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn utxo_to_account_correct_transaction() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, utxos) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from_address(receiver.address.clone(), Value(42000)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn account_to_account_correct_transaction() {
    let faucet = AddressData::account(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, _) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account_public_key(
            faucet.public_key.clone(),
            Value(1),
        ))
        .with_output(Output::from_address(receiver.address.clone(), Value(1)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn account_to_delegation_correct_transaction() {
    let faucet = AddressData::account(Discrimination::Test);
    let receiver = AddressData::delegation(Discrimination::Test);

    let (message, _) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account_public_key(
            faucet.public_key.clone(),
            Value(1),
        ))
        .with_output(Output::from_address(receiver.address.clone(), Value(1)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn delegation_to_account_correct_transaction() {
    let faucet = AddressData::delegation(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, utxos) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from_address(receiver.address.clone(), Value(42000)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn transaction_with_more_than_253_outputs() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let mut outputs = vec![];
    for _ in 0..=254 {
        let receiver = AddressData::utxo(Discrimination::Test);
        outputs.push(Output::from_address(receiver.address.clone(), Value(1)));
    }

    let (message, utxos) = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(256),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_outputs(outputs)
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        TransactionHasTooManyOutputs {
            expected: 254,
            actual: 255
        },
        ledger.apply_transaction(&signed_tx, &fees)
    )
}
