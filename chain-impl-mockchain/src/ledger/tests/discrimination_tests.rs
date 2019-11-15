#![cfg(test)]

use crate::{
    testing::{
        arbitrary::KindTypeWithoutMultisig,
        ledger::{ConfigBuilder, LedgerBuilder},
        builders::TestTxBuilder,
        data::AddressDataValue
    },
    value::Value,
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_faucet_discrimination(
    arbitrary_faucet_disc: Discrimination,
    arbitrary_faucet_address_kind: KindTypeWithoutMultisig,
    arbitrary_ledger_disc: Discrimination,
) {
    let config = ConfigBuilder::new(0)
        .with_discrimination(arbitrary_ledger_disc);

    let faucet = AddressDataValue::from_discrimination_and_kind_type(
        arbitrary_faucet_disc,
        &arbitrary_faucet_address_kind.0,
        Value(1000)
    );

    let are_discriminations_unified = arbitrary_faucet_disc == arbitrary_ledger_disc;

    match (
        are_discriminations_unified,
        LedgerBuilder::from_config(config).faucet(&faucet).build(),
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
    let faucet = AddressDataValue::from_discrimination_and_kind_type(
        arbitrary_input_disc,
        &arbitrary_input_address_kind.kind_type(),
        Value(100)
    );
    let receiver = AddressDataValue::from_discrimination_and_kind_type(
        arbitrary_output_disc,
        &arbitrary_output_address_kind.kind_type(),
        Value(100)
    );
  
    let config = ConfigBuilder::new(0)
        .with_discrimination(arbitrary_input_disc);

    let mut ledger = LedgerBuilder::from_config(config).initial_fund(&faucet).build().unwrap();
    let fragment = TestTxBuilder::new(&ledger.block0_hash).move_all_funds(&mut ledger,&faucet,&receiver).get_fragment();

    let are_discriminations_unified = arbitrary_input_disc == arbitrary_output_disc;
    let actual_result = ledger.apply_transaction(fragment);

    match (are_discriminations_unified, actual_result) {
        (true, Ok(_)) => TestResult::passed(),
        (false, Ok(_)) => {
            TestResult::error("Ledger should reject transaction with mixed discriminations")
        }
        (true, Err(err)) => {
            TestResult::error(format!("Ledger should accept transaction with unified discriminations. Err: {}",err))
        }
        (false, Err(_)) => TestResult::passed(),
    }
}
