#[cfg(test)]
#[macro_use]
extern crate quickcheck;

pub mod account;
pub mod block;
pub mod certificate;
mod date;
#[cfg(test)]
pub mod environment;
pub mod error;
pub mod key;
pub mod leadership;
pub mod ledger;
pub mod setting;
pub mod stake;
pub mod state;
pub mod transaction;
pub mod update;
pub mod value;

#[cfg(test)]
mod tests {
    use crate::ledger::Ledger;
    use crate::transaction::SignedTransaction;
    use chain_core::property::testing;
    use quickcheck::TestResult;

    quickcheck! {
        /// Randomly generated transaction should fail.
        fn prop_bad_tx_fails(l: Ledger, tx: SignedTransaction) -> TestResult {
            testing::prop_bad_transaction_fails(l, &tx)
        }
    }

}
