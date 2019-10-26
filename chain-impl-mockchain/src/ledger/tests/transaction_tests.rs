#![cfg(test)]

use crate::fee::FeeAlgorithm;
use crate::{
    accounting::account::LedgerError::NonExistent,
    fragment::Fragment,
    ledger::{self, check::TxVerifyError, Entry, Error::{TransactionMalformed,UtxoError, Account, AccountInvalidSignature}, Ledger},
    testing::{
        ConfigBuilder, LedgerBuilder,
        arbitrary::{
            AccountStatesVerifier, ArbitraryValidTransactionData, NonZeroValue, UtxoVerifier,
        },
        data::AddressData,
        TestTxBuilder,
        TestGen,
    },
    transaction::*,
    value::*,
    utxo::{Error::TransactionNotFound,Entry as UtxoEntry}
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

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

/*
#[test]
pub fn transaction_with_more_than_253_outputs() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let mut outputs = vec![];
    for _ in 0..=254 {
        let receiver = AddressData::utxo(Discrimination::Test);
        outputs.push(Output::from_address(receiver.address.clone(), Value(1)));
    }

    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(256),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let mut utxos = ledger.utxos();
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo_entry(utxos.next().unwrap()))
        .with_outputs(outputs)
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = TestGen::hash();

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        TransactionMalformed {
            source: TxVerifyError::TooManyOutputs {
                expected: 254,
                actual: 255
            }
        },
        ledger.apply_transaction(&fragment_id, &signed_tx, &fees)
    );
}
*/

#[test]
pub fn duplicated_account_transaction() {
    let testledger = LedgerBuilder::from_config(ConfigBuilder::new(0))
        .faucet(Value(1000))
        .build()
        .expect("cannot build test ledger");
    let ledger = testledger.ledger;
    let mut faucet = testledger.faucet.expect("faucet to be configured");

    let receiver = AddressData::utxo(Discrimination::Test);

    let ttx = TestTxBuilder::new(&faucet.block0_hash)
        .move_from_faucet(&mut faucet, &receiver.address, Value(100));

    let fragment_id = ttx.get_fragment_id();
    let tx = ttx.get_tx();
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &tx.as_slice(), &fees);

    match result {
        Err(err) => panic!("first transaction should be succesful but {}", err),
        Ok((ledger, _)) => {
            match ledger.apply_transaction(&fragment_id, &tx.as_slice(), &fees) {
                Err(ledger::Error::AccountInvalidSignature {..}) => {},
                Err(e) => panic!("duplicated transaction not accepted but unexpected error {}", e),
                Ok(_) => panic!("duplicated transaction accepted"),
            }
        }
    }
}

/*
#[test]
pub fn transaction_with_nonexisting_utxo_input() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);
    let value = Value(100);
    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        value,
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    
    let fake_utxo_entry = UtxoEntry{
        fragment_id: TestGen::hash(),
        output_index: 0u8,
        output: &faucet.make_output(value)
    };
    
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo_entry(fake_utxo_entry))
        .with_output(receiver.make_output(value))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = TestGen::hash();
    let fees = ledger.get_ledger_parameters();
    assert_err!(
        UtxoError {
            source: TransactionNotFound 
        },
        ledger.apply_transaction(&fragment_id, &signed_tx, &fees)
    );
}

#[test]
pub fn transaction_nonexisting_account_input() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let fake_account = AddressData::account(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);
    let value = Value(100);

    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        value,
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    
    let signed_tx = TransactionBuilder::new()
        .with_input(fake_account.make_input(value, None))
        .with_output(receiver.make_output(value))
        .authenticate()
        .with_witness(&block0_hash, &fake_account)
        .seal();

    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    let fees = ledger.get_ledger_parameters();
    assert_err!(
        Account { source: NonExistent },
        ledger.apply_transaction(&fragment_id, &signed_tx, &fees)
    );
}

#[test]
pub fn duplicated_account_transaction() {
    let faucet = AddressData::account(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);
    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(200),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(Value(100),None))
        .with_output(receiver.make_output(Value(100)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);
    assert!(result.is_ok(),"first transaction should be successful");
    let (ledger,_) = result.unwrap();

    assert!(ledger.apply_transaction(&fragment_id, &signed_tx, &fees).is_err());
}

#[test]
pub fn repeated_account_transaction() {
    let mut faucet = AddressData::account(Discrimination::Test);
    let receiver = AddressData::account(Discrimination::Test);
    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(200),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(Value(100),None))
        .with_output(receiver.make_output(Value(100)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);
    assert!(result.is_ok(),"first transaction should be successful");

    let (ledger,_) = result.unwrap();
    faucet.confirm_transaction();

    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(Value(100),None))
        .with_output(receiver.make_output(Value(100)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    let fees = ledger.get_ledger_parameters();
    assert!(ledger.apply_transaction(&fragment_id, &signed_tx, &fees).is_ok(),"second transaction should be successful");

}

#[test]
pub fn transaction_with_incorrect_account_spending_counter() {
    let faucet = AddressData::account(Discrimination::Test);
    let receiver = AddressData::account_with_spending_counter(Discrimination::Test, 1);
    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(200),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(Value(100),None))
        .with_output(receiver.make_output(Value(100)))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();

    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);
    assert!(result.is_ok(),"first transaction should be successful");
    let (ledger,_) = result.unwrap();

    let signed_tx = TransactionBuilder::new()
        .with_input(receiver.make_input(Value(100),None))
        .with_output(faucet.make_output(Value(100)))
        .authenticate()
        .with_witness(&block0_hash, &receiver)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();
    assert!(ledger.apply_transaction(&fragment_id, &signed_tx, &fees).is_err());
}
*/
