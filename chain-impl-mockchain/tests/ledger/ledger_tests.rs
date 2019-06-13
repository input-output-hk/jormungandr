use crate::common::address::AddressData;
use crate::common::ledger;
use crate::common::ledger::ConfigBuilder;
use crate::common::tx_builder::TransactionBuilder;
use chain_addr::Discrimination;

use chain_impl_mockchain::account::SpendingCounter;

use chain_impl_mockchain::ledger::Error::NotEnoughSignatures;
use chain_impl_mockchain::transaction::*;
use chain_impl_mockchain::value::*;

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
    let sender = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);

    let (message, utxos) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));
    let (_, ledger) = ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from(receiver.address.clone(), Value(1)))
        .finalize()
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
    let sender = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);

    let (message, utxos) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));
    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from(receiver.address.clone(), Value(42000)))
        .finalize()
        .with_utxo_witness(&block0_hash, &sender.private_key)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn utxo_to_account_correct_transaction() {
    let sender = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, utxos) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from(receiver.address.clone(), Value(42000)))
        .finalize()
        .with_utxo_witness(&block0_hash, &sender.private_key)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn account_to_account_correct_transaction() {
    let sender = AddressData::account(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, _) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account_pk(sender.public_key, Value(1)))
        .with_output(Output::from(receiver.address.clone(), Value(1)))
        .finalize()
        .with_account_witness(&block0_hash, &SpendingCounter::zero(), &sender.private_key)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn account_to_delegation_correct_transaction() {
    let sender = AddressData::account(Discrimination::Test);
    let receiver = AddressData::delegation(Discrimination::Test);

    let (message, _) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_account_pk(sender.public_key, Value(1)))
        .with_output(Output::from(receiver.address.clone(), Value(1)))
        .finalize()
        .with_account_witness(&block0_hash, &SpendingCounter::zero(), &sender.private_key)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}

#[test]
pub fn delegation_to_account_correct_transaction() {
    let sender = AddressData::delegation(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);

    let (message, utxos) =
        ledger::create_initial_transaction(Output::from(sender.address.clone(), Value(42000)));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());

    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo(utxos[0]))
        .with_output(Output::from(receiver.address.clone(), Value(42000)))
        .finalize()
        .with_utxo_witness(&block0_hash, &sender.private_key)
        .seal();

    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&signed_tx, &fees).is_ok());
}
