use blockcfg::property::Ledger;
use blockcfg::property::Transaction;
use quickcheck::Arbitrary;

/// Check if incorrectly generated transaction
/// will fail to be processed.
pub fn prop_bad_transactions_fails<L>(ledger: L, transaction: L::Transaction) -> bool
where
    L: Ledger + Arbitrary,
    L::Transaction: Transaction + Arbitrary,
{
    ledger.diff_transaction(&transaction).is_err()
}
