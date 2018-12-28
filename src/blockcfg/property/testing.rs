use blockcfg::property::Ledger;
use blockcfg::property::Transaction;
use quickcheck::Arbitrary;

#[derive(Debug, Copy, Clone)]
pub struct LedgerWithValidTransaction<L, T>(pub L, pub T);

pub fn prop_good_transactions_succeed<L>(input: LedgerWithValidTransaction<L, L::Transaction>)
where
    L: Ledger + Arbitrary,
{
    match input.0.diff_transaction(&input.1) {
        Err(e) => panic!("error {:#?}", e),
        Ok(_) => (),
    }
}

/// Check if incorrectly generated transaction
/// will fail to be processed.
pub fn prop_bad_transactions_fails<L>(ledger: L, transaction: L::Transaction) -> bool
where
    L: Ledger + Arbitrary,
    L::Transaction: Transaction + Arbitrary,
{
    ledger.diff_transaction(&transaction).is_err()
}
