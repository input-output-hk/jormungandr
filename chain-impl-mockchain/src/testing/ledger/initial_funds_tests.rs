#![cfg(test)]

use crate::testing::common::{
    arbitrary::OutputsWithoutMultisig,
    ledger::{self, ConfigBuilder},
};
use chain_addr::{Address, Discrimination};
use crate::{transaction::Output, value::Value};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_value_of_initial_funds(
    mut arbitrary_outputs: OutputsWithoutMultisig,
) -> TestResult {
    arbitrary_outputs.set_discrimination(Discrimination::Test);
    let (message, _) = ledger::create_initial_transactions(&arbitrary_outputs.0);
    let result = ledger::create_initial_fake_ledger(
        &[message],
        ConfigBuilder::new()
            .with_discrimination(Discrimination::Test)
            .build(),
    );
    let should_ledger_fail = should_ledger_fail(&arbitrary_outputs.0);
    match (should_ledger_fail, result) {
        (false, Ok(_)) => TestResult::passed(),
        (false, Err(err)) => TestResult::error(format!(
            "Ledger should NOT fail because there is NO zero value in initial funds: {:?}",
            err
        )),
        (true, Err(_)) => TestResult::passed(),
        (true, Ok(_)) => {
            TestResult::error("Ledger should fail because there is a zero value in initial funds")
        }
    }
}

fn should_ledger_fail(outputs: &Vec<Output<Address>>) -> bool {
    if outputs.is_empty() {
        return false;
    }
    outputs.iter().any(|x| x.value == Value::zero())
}
