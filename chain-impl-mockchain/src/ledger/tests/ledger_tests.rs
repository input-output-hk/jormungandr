#![cfg(test)]

use crate::{
    fragment::{Fragment, FragmentId},
    ledger::{
        Entry,
        Error::{NotEnoughSignatures, TransactionHasTooManyOutputs},
        Ledger,
    },
    transaction::*,
    value::*,
    testing::{ 
        address::AddressData,
        arbitrary::{ArbitraryValidTransactionData, NonZeroValue, AccountStatesVerifier},
        ledger::{self,ConfigBuilder},
        tx_builder::TransactionBuilder,
    }
};
use chain_addr::Discrimination;
use chain_core::property::Fragment as _;
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
    let message = ledger::create_initial_transaction(Output::from_address(
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
    let fragment_id = Fragment::Transaction(signed_tx.clone()).id();

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
pub fn total_funds_are_total_in_ledger(
    mut transaction_data: ArbitraryValidTransactionData,
) -> TestResult {
    let message =
        ledger::create_initial_transactions(&transaction_data.make_outputs_from_all_addresses());
    let (block0_hash, ledger) = ledger::create_initial_fake_ledger(
        &[message],
        ConfigBuilder::new()
            .with_discrimination(Discrimination::Test)
            .build(),
    )
    .expect("ledger_failed");

    let inputs = transaction_data.make_inputs(&ledger);
    let outputs = transaction_data.make_outputs();
    let input_addresses = transaction_data.input_addresses();

    let signed_tx = TransactionBuilder::new()
        .with_inputs(inputs)
        .with_outputs(outputs)
        .authenticate()
        .with_witnesses(&block0_hash, &input_addresses)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).id();

    let total_funds_before = calculate_total_funds_in_ledger(&ledger);
    let fees = ledger.get_ledger_parameters();
    let result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);

    match result {
        Err(err) => TestResult::error(format!("Error from ledger: {:?}", err)),

        Ok((ledger,_)) =>  {
            let total_funds_after = calculate_total_funds_in_ledger(&ledger);
            if total_funds_before != total_funds_after {
                return TestResult::error(format!(
                    "Total funds in ledger before and after transaction is not equal {} <> {} ",
                    total_funds_before, total_funds_after))
            }
            let account_state_verifier = AccountStatesVerifier::new(transaction_data);
            let account_state_verification_result = account_state_verifier.verify(ledger.accounts());
            if account_state_verification_result.is_err(){
                return TestResult::error(format!("{}",account_state_verification_result.err().unwrap()))
            }
            TestResult::passed()
        }
    }
}

#[test]
pub fn utxo_no_enough_signatures() {
    let faucet = AddressData::utxo(Discrimination::Test);
    let receiver = AddressData::utxo(Discrimination::Test);

    let message = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));
    let (_, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();
    let mut utxos = ledger.utxos();
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo_entry(utxos.next().unwrap()))
        .with_output(Output::from_address(receiver.address.clone(), Value(1)))
        .authenticate()
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).id();

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        NotEnoughSignatures {
            actual: 0,
            expected: 1
        },
        ledger.apply_transaction(&fragment_id, &signed_tx, &fees)
    )
}

#[test]
pub fn transaction_with_more_than_253_outputs() {
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
    let signed_tx = TransactionBuilder::new()
        .with_input(Input::from_utxo_entry(utxos.next().unwrap()))
        .with_outputs(outputs)
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    // here we have to build a random FragmentId, since the transaction is invalid,
    // and will trigger an assert during Fragment -> Id calculation otherwise
    let fragment_id = FragmentId::hash_bytes(&[1, 2, 3]);

    let fees = ledger.get_ledger_parameters();
    assert_err!(
        TransactionHasTooManyOutputs {
            expected: 254,
            actual: 255
        },
        ledger.apply_transaction(&fragment_id, &signed_tx, &fees)
    )
}

#[test]
pub fn iterate() {
    let faucet = AddressData::utxo(Discrimination::Test);

    let message = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(42000),
    ));
    let (_block0_hash, ledger) =
        ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build()).unwrap();

    // FIXME: generate arbitrary ledger

    for item in ledger.iter() {
        match item {
            Entry::Globals(globals) => {
                println!(
                    "Globals date={} length={} block0_hash={} start_time={:?} discr={} kes_update_speed={}",
                    globals.date,
                    globals.chain_length,
                    globals.static_params.block0_initial_hash,
                    globals.static_params.block0_start_time,
                    globals.static_params.discrimination,
                    globals.static_params.kes_update_speed,
                );
            }
            Entry::Utxo(entry) => {
                println!(
                    "Utxo {} {} {}",
                    entry.fragment_id, entry.output_index, entry.output
                );
            }
            Entry::OldUtxo(entry) => {
                println!(
                    "OldUtxo {} {} {}",
                    entry.fragment_id, entry.output_index, entry.output
                );
            }
            Entry::Account((id, state)) => {
                println!(
                    "Account {} {} {:?} {}",
                    id,
                    u32::from(state.counter),
                    state.delegation,
                    state.value,
                );
            }
            Entry::ConfigParam(param) => {
                println!(
                    "ConfigParam {:?} {:?}",
                    crate::config::Tag::from(&param),
                    param,
                );
            }
            Entry::UpdateProposal((id, state)) => {
                println!(
                    "UpdateProposal {} {:?} {} {:?}",
                    id, state.proposal, state.proposal_date, state.votes
                );
            }
            Entry::MultisigAccount((id, state)) => {
                println!(
                    "MultisigAccount {} {} {:?} {}",
                    id,
                    u32::from(state.counter),
                    state.delegation,
                    state.value,
                );
            }
            Entry::MultisigDeclaration((id, decl)) => {
                println!(
                    "MultisigDeclaration {} {} {}",
                    id,
                    decl.threshold(),
                    decl.total(),
                );
            }
            Entry::StakePool((id, info)) => {
                println!(
                    "StakePool {} {} {:?} {:?}",
                    id, info.serial, info.owners, info.initial_key,
                );
            }
        }
    }

    let ledger2: Result<Ledger, _> = ledger.iter().collect();
    let ledger2 = ledger2.unwrap();

    assert!(ledger == ledger2);
}
