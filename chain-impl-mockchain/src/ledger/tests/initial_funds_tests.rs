#![cfg(test)]

use crate::{
    testing::{
        ledger::{ConfigBuilder, LedgerBuilder},
        arbitrary::address::ArbitraryAddressDataValueVec,
        data::AddressDataValue
    },
    value::Value,
};
use chain_addr::Discrimination;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_value_of_initial_funds(
    arbitrary_faucets: ArbitraryAddressDataValueVec,
) -> TestResult {
    let config = ConfigBuilder::new(0)
        .with_discrimination(Discrimination::Test);

    TestResult::from_bool(LedgerBuilder::from_config(config)
        .initial_funds(&arbitrary_faucets.values())
        .build()
        .is_ok()
    )
}

#[test]
pub fn ledger_fails_to_start_when_there_is_zero_output() {
    let config = ConfigBuilder::new(0)
        .with_discrimination(Discrimination::Test);

    let address = AddressDataValue::account(Discrimination::Test,Value::zero());

    assert!(LedgerBuilder::from_config(config)
        .faucet(&address)
        .build()
        .is_err(),
        "Ledger should fail to start with zero value output");
}