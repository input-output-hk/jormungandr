use std::collections::BTreeMap;

use chain_addr::Address;
use chain_core::property::Update;

use crate::transaction::{Output, UtxoPointer};

/// Diff between the 2 state of the blockchain.
///
/// This structure has the property to be compatible with rollback
/// principles.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Diff {
    /// these are the diff for the transaction
    pub transactions_diff: TransactionsDiff,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TransactionsDiff {
    /// These are the new spent output, the TxOut is not present
    /// on the transaction but it is kept here to keep the diff
    /// of what is being removed from the ledge.
    pub spent_outputs: BTreeMap<UtxoPointer, Output<Address>>,

    /// these are the new UTxO that the Diff is adding to the new
    /// state of the blockchain
    pub new_unspent_outputs: BTreeMap<UtxoPointer, Output<Address>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueDiff<T> {
    None,
    Replace(T, T),
}

impl<T> ValueDiff<T>
where
    T: Eq,
{
    pub fn check(&self, dest: &T) -> bool {
        match &self {
            ValueDiff::None => true,
            ValueDiff::Replace(old, _) => dest == old,
        }
    }

    /// Apply this diff to a destination, overwriting it with the new
    /// value if it is equal to the expected old value. Panic if the
    /// old value is unexpected. (The caller is expected to use
    /// `check` first to validate the expected state of all values in
    /// an update first. We panic to ensure that we don't end up in a
    /// half-update state.)
    pub fn apply_to(self, dest: &mut T) {
        match self {
            ValueDiff::None => {}
            ValueDiff::Replace(old, new) => {
                assert!(dest == &old);
                *dest = new;
            }
        }
    }
}

impl<T: PartialEq> ValueDiff<T> {
    pub fn inverse(self) -> Self {
        match self {
            ValueDiff::None => ValueDiff::None,
            ValueDiff::Replace(a, b) => ValueDiff::Replace(b, a),
        }
    }

    pub fn union(&mut self, other: Self) -> &mut Self {
        match (std::mem::replace(self, ValueDiff::None), other) {
            (ValueDiff::None, ValueDiff::None) => {}
            (ValueDiff::None, ValueDiff::Replace(c, d)) => {
                *self = ValueDiff::Replace(c, d);
            }
            (ValueDiff::Replace(a, b), ValueDiff::None) => {
                *self = ValueDiff::Replace(a, b);
            }
            (ValueDiff::Replace(a, b), ValueDiff::Replace(c, d)) => {
                assert!(b == c);
                if a == d {
                    *self = ValueDiff::None;
                } else {
                    *self = ValueDiff::Replace(a, d);
                }
            }
        }
        self
    }
}

impl Update for Diff {
    fn empty() -> Self {
        Diff {
            transactions_diff: TransactionsDiff::empty(),
        }
    }
    fn inverse(self) -> Self {
        Diff {
            transactions_diff: self.transactions_diff.inverse(),
        }
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.transactions_diff.union(other.transactions_diff);
        self
    }
}

impl Update for TransactionsDiff {
    fn empty() -> Self {
        TransactionsDiff {
            spent_outputs: BTreeMap::new(),
            new_unspent_outputs: BTreeMap::new(),
        }
    }

    fn inverse(self) -> Self {
        TransactionsDiff {
            spent_outputs: self.new_unspent_outputs,
            new_unspent_outputs: self.spent_outputs,
        }
    }

    fn union(&mut self, other: Self) -> &mut Self {
        // 1. other might be spending outputs that were _new_ in self
        //    we need to remove them first.
        for other_spending in other.spent_outputs.into_iter() {
            if let Some(_) = self.new_unspent_outputs.remove(&other_spending.0) {
                // just ignore the deleted output
            } else {
                self.spent_outputs
                    .insert(other_spending.0, other_spending.1);
            }
        }

        // 2. other might be spending outputs that were _new_ in self
        for other_output in other.new_unspent_outputs.into_iter() {
            if let Some(_) = self.spent_outputs.remove(&other_output.0) {
                // just ignore and drop the value
            } else {
                self.new_unspent_outputs
                    .insert(other_output.0, other_output.1);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_core::property::testing;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Diff {
        fn arbitrary<G: Gen>(g: &mut G) -> Diff {
            Diff {
                transactions_diff: Arbitrary::arbitrary(g),
            }
        }
    }
    impl Arbitrary for TransactionsDiff {
        fn arbitrary<G: Gen>(g: &mut G) -> TransactionsDiff {
            TransactionsDiff {
                spent_outputs: Arbitrary::arbitrary(g),
                new_unspent_outputs: Arbitrary::arbitrary(g),
            }
        }
    }

    quickcheck! {
        /*
        FIXME: add tests for checking associativity of diffs on
        randomly generated values of the type we're diffing.

        fn diff_union_is_associative(types: (Diff, Diff, Diff)) -> bool {
            testing::update_associativity(types.0, types.1, types.2)
        }
        */
        fn diff_union_has_identity_element(diff: Diff) -> bool {
            testing::update_identity_element(diff)
        }
        fn diff_union_has_inverse_element(diff: Diff) -> bool {
            testing::update_inverse_element(diff)
        }

        /*
        FIXME: add tests for checking associativity of diffs on
        randomly generated values of the type we're diffing.

        fn transactions_diff_union_is_associative(types: (TransactionsDiff, TransactionsDiff, TransactionsDiff)) -> bool {
            testing::update_associativity(types.0, types.1, types.2)
        }
        fn transactions_diff_union_has_identity_element(transactions_diff: TransactionsDiff) -> bool {
            testing::update_identity_element(transactions_diff)
        }
        fn transactions_diff_union_has_inverse_element(transactions_diff: TransactionsDiff) -> bool {
            testing::update_inverse_element(transactions_diff)
        }
        fn transactions_diff_union_is_commutative(types: (TransactionsDiff, TransactionsDiff)) -> bool {
            testing::update_union_commutative(types.0, types.1)
        }
        */
    }
}
