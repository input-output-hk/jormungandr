/// Ledger for the blockchain, maintain the validity of the transactions
/// happening within the blockchain.
///
/// This trait's model separates 2 actions we may want to do with the
/// blockchain when analyzing the blocks:
///
/// 1. build a diff, an update, of what the transaction will change within
///    the state of the ledger.
/// 2. apply a diff on the ledger, update its state.
///
/// This model will allow us to do some testing. But also to monitor the
/// changes applied to the ledger.
///
/// Also this way we can allow for a function way of storing the ledger
/// state: storing diff by diff the state so we can easily perform
/// roll backs (simply reloading the state but ignoring the last ones
/// up to the rollback point).
///
pub trait Ledger {
    /// this is the kind of transactions the implementor will be interested
    /// about.
    type Transaction: Transaction;

    /// a diff to apply on the ledger to modify the ledger's state.
    type Diff;

    /// the kind of error we ought to expect when applying the different
    /// operations of the ledger.
    type Error: std::error::Error;

    /// construct a diff between the current state of the ledger and the given
    /// transaction.
    ///
    /// This function must verify the transaction is valid within itself
    /// and that the transaction's inputs are present in the UTxO database
    /// of the ledger.
    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error>;

    /// just like `diff_transaction` but returns the diff for all the given
    /// transactions.
    ///
    /// The diff here is an accumulated diff.
    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a;

    /// add/apply a diff to the given ledger.
    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error>;

    /// this is a convenient function to both diff and apply the diff
    /// of the given transactions.
    fn update<'a, I>(&mut self, transactions: I) -> Result<&mut Self, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a
    {
        let diff = self.diff(transactions)?;
        self.add(diff)
    }
}

/// define the needed properties of a given transaction.
///
/// A transactions is composed of Inputs and Outputs
///
/// This is fine for UTxO based blockchain. We might need to update
/// this trait to allow for account based blockchain.
pub trait Transaction {
    /// here is the type of the Input
    type Input;
    /// here is the type of the Output
    type Output;
    type Id;

    fn id(&self) -> Self::Id;
}

impl<'a, T: Transaction> Transaction for &'a T {
    type Input = <T as Transaction>::Input;
    type Output = <T as Transaction>::Output;
    type Id = <T as Transaction>::Id;

    fn id(&self) -> Self::Id { (*self).id() }
}

/// accessor to a trait with `Transactions` in it. Transactions that can
/// be used by a Ledger.
///
/// This trait simply provides a generic way to access all the transactions
/// of a block in the chain.
pub trait HasTransaction
{
    /// the transaction Type.
    type Transaction: Transaction;

    /// access all the transactions of the implementor via the returned
    /// iterator.
    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction>;
}
impl<'b, B: HasTransaction> HasTransaction for &'b B {
    type Transaction = <B as HasTransaction>::Transaction;

    fn transactions<'a>(&'a self) -> std::slice::Iter<'a, Self::Transaction> {
        (*self).transactions()
    }
}