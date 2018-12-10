pub trait Ledger {
    type Transaction: Transaction;
    type Diff;

    type Error: std::error::Error;

    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error>;

    fn verify_transaction(&self, transaction: &Self::Transaction) -> Result<bool, Self::Error>;

    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a;

    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error>;

    fn update<'a, I>(&mut self, transactions: I) -> Result<&mut Self, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a
    {
        let diff = self.diff(transactions)?;
        self.add(diff)
    }
}

pub trait Transaction {
    type Input;
    type Output;
    type Id;

    fn id(&self) -> Self::Id;
}

pub trait HasTransaction<'a> {
    type Transaction: 'a + Transaction;
    type TransactionIterator: Iterator<Item = &'a Self::Transaction>;

    fn transactions(&'a self) -> Self::TransactionIterator;
}