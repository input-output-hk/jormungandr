use crate::common::address::AddressData;
use crate::common::arbitrary::transaction::*;
use crate::common::ledger;
use crate::common::ledger::ConfigBuilder;
use chain_addr::Address;
use chain_impl_mockchain::transaction::Output;
use chain_impl_mockchain::value::Value;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

#[quickcheck]
pub fn ledger_verifies_value_of_initial_funds(arbitrary_outputs: ArbitraryOutputs) -> TestResult {
    let (message, _) = ledger::create_initial_transactions(arbitrary_outputs.outputs());
    let result = ledger::create_initial_fake_ledger(&[message], ConfigBuilder::new().build());
    let should_ledger_fail = should_ledger_fail(arbitrary_outputs.outputs());
    match (should_ledger_fail, result) {
        (false, Ok(_)) => TestResult::passed(),
        (false, Err(_)) => TestResult::error(
            "Ledger should NOT fail because there is NO zero value in initial funds",
        ),
        (true, Err(_)) => TestResult::passed(),
        (true, Ok(_)) => {
            TestResult::error("Ledger should fail because there is a zero value in initial funds")
        }
    }
}

fn should_ledger_fail(outputs: Vec<Output<Address>>) -> bool {
    if outputs.is_empty() {
        return false;
    }
    outputs.iter().any(|x| x.value == Value::zero())
}
