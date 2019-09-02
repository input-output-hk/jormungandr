#![cfg(test)]

use crate::fragment::Fragment;
use crate::testing::{
    data::AddressData,
    arbitrary::KindTypeWithoutMultisig,
    ledger::{self, ConfigBuilder},
    tx_builder::TransactionBuilder,
};
use crate::transaction::*;
use crate::value::*;
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_faucet_discrimination(
    arbitrary_faucet_disc: Discrimination,
    arbitrary_faucet_address_kind: KindTypeWithoutMultisig,
    arbitrary_ledger_disc: Discrimination,
) {
    let faucet = AddressData::from_discrimination_and_kind_type(
        arbitrary_faucet_disc,
        &arbitrary_faucet_address_kind.0,
    );

    let message = ledger::create_initial_transaction(Output::from_address(
        faucet.address.clone(),
        Value(100),
    ));

    let are_discriminations_unified = arbitrary_faucet_disc == arbitrary_ledger_disc;

    let config = ConfigBuilder::new()
        .with_discrimination(arbitrary_ledger_disc)
        .build();
    match (
        are_discriminations_unified,
        ledger::create_initial_fake_ledger(&[message], config),
    ) {
        (true, Ok(_)) => TestResult::passed(),
        (false, Ok(_)) => {
            TestResult::error("Ledger should reject transaction with mixed discriminations")
        }
        (true, Err(_)) => {
            TestResult::error("Ledger should accept transaction with unified discriminations")
        }
        (false, Err(_)) => TestResult::passed(),
    };
}

#[quickcheck]
pub fn ledger_verifies_transaction_discrimination(
    arbitrary_input_disc: Discrimination,
    arbitrary_output_disc: Discrimination,
    arbitrary_input_address_kind: KindTypeWithoutMultisig,
    arbitrary_output_address_kind: KindTypeWithoutMultisig,
) -> TestResult {
    let faucet = AddressData::from_discrimination_and_kind_type(
        arbitrary_input_disc,
        &arbitrary_input_address_kind.kind_type(),
    );
    let receiver = AddressData::from_discrimination_and_kind_type(
        arbitrary_output_disc,
        &arbitrary_output_address_kind.kind_type(),
    );
    let value = Value(100);
    let message =
        ledger::create_initial_transaction(Output::from_address(faucet.address.clone(), value));

    let config = ConfigBuilder::new()
        .with_discrimination(arbitrary_input_disc)
        .build();
    let (block0_hash, ledger) = ledger::create_initial_fake_ledger(&[message], config).unwrap();
    let mut utxos = ledger.utxos();
    let signed_tx = TransactionBuilder::new()
        .with_input(faucet.make_input(value, utxos.next()))
        .with_output(Output::from_address(receiver.address.clone(), value))
        .authenticate()
        .with_witness(&block0_hash, &faucet)
        .seal();
    let fragment_id = Fragment::Transaction(signed_tx.clone()).hash();

    let are_discriminations_unified = arbitrary_input_disc == arbitrary_output_disc;

    let fees = ledger.get_ledger_parameters();
    let actual_result = ledger.apply_transaction(&fragment_id, &signed_tx, &fees);

    match (are_discriminations_unified, actual_result) {
        (true, Ok(_)) => TestResult::passed(),
        (false, Ok(_)) => {
            TestResult::error("Ledger should reject transaction with mixed discriminations")
        }
        (true, Err(_)) => {
            TestResult::error("Ledger should accept transaction with unified discriminations")
        }
        (false, Err(_)) => TestResult::passed(),
    }
}
