#![cfg(test)]

use crate::{
    ledger::check::CHECK_TX_MAXIMUM_INPUTS,
    testing::{
        arbitrary::address::ArbitraryAddressDataValueVec,
        data::AddressDataValue,
        ledger::{ConfigBuilder, LedgerBuilder},
    },
    value::Value,
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;
use std::iter;

#[quickcheck]
pub fn ledger_verifies_value_of_initial_funds(
    arbitrary_faucets: ArbitraryAddressDataValueVec,
) -> TestResult {
    let config = ConfigBuilder::new(0).with_discrimination(Discrimination::Test);

    TestResult::from_bool(
        LedgerBuilder::from_config(config)
            .initial_funds(&arbitrary_faucets.values())
            .build()
            .is_ok(),
    )
}

#[test]
pub fn ledger_fails_to_start_when_there_is_zero_output() {
    let config = ConfigBuilder::new(0).with_discrimination(Discrimination::Test);

    let address = AddressDataValue::account(Discrimination::Test, Value::zero());

    assert!(
        LedgerBuilder::from_config(config)
            .faucet(&address)
            .build()
            .is_err(),
        "Ledger should fail to start with zero value output"
    );
}

#[test]
#[should_panic]
pub fn ledger_fails_to_start_when_there_are_more_than_255_initials() {
    let config = ConfigBuilder::new(0).with_discrimination(Discrimination::Test);
    let addresses: Vec<AddressDataValue> =
        iter::from_fn(|| Some(AddressDataValue::account(Discrimination::Test, Value(10))))
            .take(CHECK_TX_MAXIMUM_INPUTS as usize + 1)
            .collect();
    LedgerBuilder::from_config(config)
        .faucets(&addresses)
        .build()
        .unwrap();
}
