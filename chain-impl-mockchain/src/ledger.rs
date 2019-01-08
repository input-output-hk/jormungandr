//! Mockchain ledger. Ledger exists in order to update the
//! current state and verify transactions.

use crate::error::*;
use crate::transaction::*;
use chain_core::property;
use std::collections::HashMap;

/// Basic ledger structure. Ledger is represented as the
/// state of unspent output values, associated with their
/// owner.
#[derive(Debug, Clone)]
pub struct Ledger {
    unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl Ledger {
    pub fn new() -> Self {
        Ledger {
            unspent_outputs: HashMap::new(),
        }
    }
}

/// Diff of the ledger state.
#[derive(Debug, Clone)]
pub struct Diff {
    /// List of the outputs that were spent in the transaction.
    spent_outputs: HashMap<UtxoPointer, Output>,
    /// List of the new outputs that were produced by the transaction.
    new_unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: HashMap::new(),
            new_unspent_outputs: HashMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}
impl property::Ledger<SignedTransaction> for Ledger {
    type Update = Diff;
    type Error = Error;

    fn input<'a>(
        &'a self,
        input: &<self::SignedTransaction as property::Transaction>::Input,
    ) -> Result<&'a <self::SignedTransaction as property::Transaction>::Output, Self::Error> {
        match self.unspent_outputs.get(&input) {
            Some(output) => Ok(output),
            None => Err(Error::InputDoesNotResolve(*input)),
        }
    }

    fn diff_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<Self::Update, Self::Error> {
        use chain_core::property::Transaction;

        let mut diff = Diff::new();
        let id = transaction.id();
        // 1. validate transaction without looking into the context
        // and that each input is validated by the matching key.
        for (input, witness) in transaction
            .tx
            .inputs
            .iter()
            .zip(transaction.witnesses.iter())
        {
            if !witness.verifies(transaction.tx.id()) {
                return Err(Error::InvalidTxSignature(witness.clone()));
            }
            if let Some(output) = self.unspent_outputs.get(&input) {
                if !witness.matches(&output) {
                    return Err(Error::InvalidSignature(*input, *output, witness.clone()));
                }
                if let Some(output) = diff.spent_outputs.insert(*input, *output) {
                    return Err(Error::DoubleSpend(*input, output));
                }
            } else {
                return Err(Error::InputDoesNotResolve(*input));
            }
        }
        // 2. prepare to add the new outputs
        for (index, output) in transaction.tx.outputs.iter().enumerate() {
            diff.new_unspent_outputs
                .insert(UtxoPointer::new(id, index as u32), *output);
        }
        // 3. verify that transaction sum is zero.
        let spent = diff
            .spent_outputs
            .iter()
            .fold(0, |acc, (_, Output(_, Value(x)))| acc + x);
        let new_unspent = diff
            .new_unspent_outputs
            .iter()
            .fold(0, |acc, (_, Output(_, Value(x)))| acc + x);
        if spent != new_unspent {
            return Err(Error::TransactionSumIsNonZero(spent, new_unspent));
        }
        Ok(diff)
    }

    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Update, Self::Error>
    where
        I: IntoIterator<Item = &'a SignedTransaction> + Sized,
    {
        let mut diff = Diff::new();

        for transaction in transactions {
            diff.extend(self.diff_transaction(transaction)?);
        }

        Ok(diff)
    }

    fn apply(&mut self, diff: Self::Update) -> Result<&mut Self, Self::Error> {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.unspent_outputs.remove(spent_output) {
                return Err(Error::InputDoesNotResolve(*spent_output));
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(original_output) = self.unspent_outputs.insert(input, output) {
                return Err(Error::InputWasAlreadySet(input, original_output, output));
            }
        }

        Ok(self)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Ledger {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Ledger {
                unspent_outputs: Arbitrary::arbitrary(g),
            }
        }
    }

}
