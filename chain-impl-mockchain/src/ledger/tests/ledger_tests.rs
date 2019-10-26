#![cfg(test)]

use crate::fee::FeeAlgorithm;
use crate::{
    fragment::Fragment,
    ledger::{
        check::TxVerifyError,
        Entry,
        Error::TransactionMalformed,
        Ledger,
    },
    testing::{
        arbitrary::{
            AccountStatesVerifier, ArbitraryValidTransactionData, NonZeroValue, UtxoVerifier,
        },
        data::AddressData,
        ledger::{self, ConfigBuilder},
        TestGen,
    },
    transaction::*,
    value::*,
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

#[quickcheck]
pub fn ledger_accepts_correct_transaction(
    faucet: AddressData,
    receiver: AddressData,
    value: NonZeroValue,
) -> TestResult {
    let message = requests::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        value.into(),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let mut utxos = ledger.utxos();
    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(value.into(), utxos.next()))
        .with_output(Output::from_address(receiver.address.clone(), value.into()))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();

    let total_funds_before = calculate_total_funds_in_ledger(&ledger);

    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);

    match result {
        Err(err) => TestResult::error(format!("Error from ledger: {}", err)),
        Ok((ledger, _)) => {
            let total_funds_after = calculate_total_funds_in_ledger(&ledger);
            match total_funds_before == total_funds_after {
                false => TestResult::error(format!(
                    "Total funds in ledger before and after transaction is not equal {} <> {} ",
                    total_funds_before, total_funds_after
                )),
                true => TestResult::passed(),
            }
        }
    }
}

fn calculate_total_funds_in_ledger(ledger: &Ledger) -> u64 {
    ledger.utxos().map(|x| x.output.value.0).sum::<u64>()
        + ledger.accounts().get_total_value().unwrap().0
}

#[quickcheck]
pub fn total_funds_are_const_in_ledger(
    mut transaction_data: ArbitraryValidTransactionData,
) -> TestResult {
    let message =
        ledger::create_initial_transactions(&transaction_data.make_outputs_from_all_addresses());
    let (block0_hash, ledger) = ledger::create_initial_fake_ledger(
        &[message],
        ConfigBuilder::new()
            .with_discrimination(Discrimination::Test)
            .with_fee(transaction_data.fee.clone())
            .build(),
    )
    .expect("ledger_failed");

    let inputs = transaction_data.make_inputs(&ledger);
    let outputs = transaction_data.make_outputs();
    let input_addresses = transaction_data.input_addresses();

    let signed_tx = TxBuilder::new()
        .set_nopayload()
        .set_ios(&inputs, &outputs)
        .authenticate()
        .with_witnesses(&block0_hash, &input_addresses)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();

    let total_funds_before = calculate_total_funds_in_ledger(&ledger);
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);

    match result {
        Err(err) => TestResult::error(format!("Error from ledger: {:?}", err)),

        Ok((ledger, _)) => {
            let total_funds_after = calculate_total_funds_in_ledger(&ledger);
            let fee = transaction_data
                .fee
                .calculate_tx(&signed_tx.transaction)
                .unwrap()
                .0;

            if total_funds_before != (total_funds_after + fee) {
                return TestResult::error(format!(
                    "Total funds in ledger before and (after transaction + fee) is not equal {} <> {} (fee: {:?})",
                    total_funds_before, (total_funds_after + fee),transaction_data.fee
                ));
            }

            let utxo_verifier = UtxoVerifier::new(transaction_data.clone());
            let utxo_verification_result = utxo_verifier.verify(&ledger);
            if utxo_verification_result.is_err() {
                return TestResult::error(format!("{}", utxo_verification_result.err().unwrap()));
            }

            let account_state_verifier = AccountStatesVerifier::new(transaction_data.clone());
            let account_state_verification_result =
                account_state_verifier.verify(ledger.accounts());
            if account_state_verification_result.is_err() {
                return TestResult::error(format!(
                    "{}",
                    account_state_verification_result.err().unwrap()
                ));
            }
            TestResult::passed()
        }
    }
}

#[test]
pub fn transaction_with_255_outputs() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let mut outputs = vec![];
    for _ in 0..=254 {
        let receiver = AddressData::utxo(Discrimination::Test);
        outputs.push(Output::from_address(receiver.address.clone(), Value(1)));
    }

    let message = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(256),
    ));

    let (block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let mut utxos = ledger.utxos();
    let signed_tx = TxBuilder::new()
        .set_nopayload()
        .set_ios(&[Input::from_utxo_entry(utxos.next().unwrap())], &outputs)
        .set_witnesses(&[])
        // .with_witness(&block0_hash, &faucet)
        .set_payload_auth(&());
    let fragment_id = TestGen::hash();

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        TransactionMalformed {
            source: TxVerifyError::TooManyOutputs {
                expected: 254,
                actual: 255
            }
        },
        ledger.apply_transaction(&fragment_id, &signed_tx.as_slice(), &fees)
    );
}
