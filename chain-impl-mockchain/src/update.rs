use std::collections::BTreeMap;

use chain_core::property::Update;

use crate::key::Hash;
use crate::transaction::{Output, UtxoPointer};

/// Diff between the 2 state of the blockchain.
///
/// This structure has the property to be compatible with rollback
/// principles.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Diff {
    /// these are the diff for the transaction
    pub transactions_diff: TransactionsDiff,

    /// settings diff
    pub settings_diff: SettingsDiff,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TransactionsDiff {
    /// These are the new spent output, the TxOut is not present
    /// on the transaction but it is kept here to keep the diff
    /// of what is being removed from the ledge.
    pub spent_outputs: BTreeMap<UtxoPointer, Output>,

    /// these are the new UTxO that the Diff is adding to the new
    /// state of the blockchain
    pub new_unspent_outputs: BTreeMap<UtxoPointer, Output>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueDiff<T> {
    None,
    Replace(T, T),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SettingsDiff {
    pub block_id: ValueDiff<Hash>,
}

impl<T: PartialEq> ValueDiff<T> {
    fn inverse(self) -> Self {
        match self {
            ValueDiff::None => ValueDiff::None,
            ValueDiff::Replace(a, b) => ValueDiff::Replace(b, a),
        }
    }

    fn union(&mut self, other: Self) -> &mut Self {
        match (std::mem::replace(self, ValueDiff::None), other) {
            (ValueDiff::None, ValueDiff::None) => {}
            (ValueDiff::None, ValueDiff::Replace(c, d)) => {
                std::mem::replace(self, ValueDiff::Replace(c, d));
            }
            (ValueDiff::Replace(a, b), ValueDiff::None) => {
                std::mem::replace(self, ValueDiff::Replace(a, b));
            }
            (ValueDiff::Replace(a, _b), ValueDiff::Replace(_c, d)) => {
                if a == d {
                    std::mem::replace(self, ValueDiff::None);
                } else {
                    std::mem::replace(self, ValueDiff::Replace(a, d));
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
            settings_diff: SettingsDiff::empty(),
        }
    }
    fn inverse(self) -> Self {
        Diff {
            transactions_diff: self.transactions_diff.inverse(),
            settings_diff: self.settings_diff.inverse(),
        }
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.transactions_diff.union(other.transactions_diff);
        self.settings_diff.union(other.settings_diff);
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

impl Update for SettingsDiff {
    fn empty() -> Self {
        SettingsDiff {
            block_id: ValueDiff::None,
        }
    }
    fn inverse(self) -> Self {
        SettingsDiff {
            block_id: self.block_id.inverse(),
        }
    }
    fn union(&mut self, other: Self) -> &mut Self {
        self.block_id.union(other.block_id);
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
                settings_diff: Arbitrary::arbitrary(g),
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
    impl Arbitrary for SettingsDiff {
        fn arbitrary<G: Gen>(g: &mut G) -> SettingsDiff {
            SettingsDiff {
                block_id: ValueDiff::Replace(Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)),
            }
        }
    }

    quickcheck! {
        fn diff_union_is_associative(types: (Diff, Diff, Diff)) -> bool {
            testing::update_associativity(types.0, types.1, types.2)
        }
        fn diff_union_has_identity_element(diff: Diff) -> bool {
            testing::update_identity_element(diff)
        }
        fn diff_union_has_inverse_element(diff: Diff) -> bool {
            testing::update_inverse_element(diff)
        }

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

        fn settings_diff_union_is_associative(types: (SettingsDiff, SettingsDiff, SettingsDiff)) -> bool {
            testing::update_associativity(types.0, types.1, types.2)
        }
        fn settings_diff_union_has_identity_element(settings_diff: SettingsDiff) -> bool {
            testing::update_identity_element(settings_diff)
        }
        fn settings_diff_union_has_inverse_element(settings_diff: SettingsDiff) -> bool {
            testing::update_inverse_element(settings_diff)
        }
    }
}
