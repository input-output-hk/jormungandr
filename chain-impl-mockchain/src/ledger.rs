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
    pub unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl Ledger {
    pub fn new(input: HashMap<UtxoPointer, Output>) -> Self {
        Ledger {
            unspent_outputs: input,
        }
    }
}

/// Diff of the ledger state.
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    /// List of the outputs that were spent in the transaction.
    spent_outputs: HashMap<UtxoPointer, Output>,
    /// List of the new outputs that were produced by the transaction.
    new_unspent_outputs: HashMap<UtxoPointer, Output>,
}
impl property::Update for Diff {
    fn empty() -> Self {
        Diff {
            spent_outputs: HashMap::new(),
            new_unspent_outputs: HashMap::new(),
        }
    }

    fn inverse(self) -> Self {
        Diff {
            spent_outputs: self.new_unspent_outputs,
            new_unspent_outputs: self.spent_outputs,
        }
    }

    fn union(mut self, other: Self) -> Self {
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

        let mut diff = <Diff as property::Update>::empty();
        let id = transaction.id();
        // 0. verify that number of signatures matches number of
        // transactions
        if transaction.tx.inputs.len() > transaction.witnesses.len() {
            return Err(Error::NotEnoughSignatures(
                transaction.tx.inputs.len(),
                transaction.witnesses.len(),
            ));
        }
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
    use crate::address::Address;
    use crate::key::{Hash, PrivateKey};
    use cardano::hdwallet as crypto;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Diff {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Diff {
                spent_outputs: Arbitrary::arbitrary(g),
                new_unspent_outputs: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for Ledger {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Ledger {
                unspent_outputs: Arbitrary::arbitrary(g),
            }
        }
    }

    fn make_key(u: u8) -> (PrivateKey, Address) {
        let pk1 = PrivateKey::normalize_bytes([u; crypto::XPRV_SIZE]);
        let user_address = Address::new(&pk1.public());
        (pk1, user_address)
    }

    #[test]
    pub fn tx_no_witness() -> () {
        use chain_core::property::Ledger;
        let (_pk1, user1_address) = make_key(0);
        let tx0_id = TransactionId(Hash::hash_bytes(&[0]));
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user1_address, Value(1)))]
                .iter()
                .cloned()
                .collect(),
        );
        let tx = Transaction {
            inputs: vec![utxo0],
            outputs: vec![Output(user1_address, Value(1))],
        };
        let signed_tx = SignedTransaction {
            tx: tx,
            witnesses: vec![],
        };
        assert_eq!(
            Err(Error::NotEnoughSignatures(1, 0)),
            ledger.diff_transaction(&signed_tx)
        )
    }

    #[test]
    pub fn tx_wrong_witness() -> () {
        use chain_core::property::Ledger;
        use chain_core::property::Transaction;
        let (_, user0_address) = make_key(0);
        let tx0_id = TransactionId(Hash::hash_bytes(&[0]));
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user0_address, Value(1)))]
                .iter()
                .cloned()
                .collect(),
        );
        let output0 = Output(user0_address, Value(1));
        let tx = crate::transaction::Transaction {
            inputs: vec![utxo0],
            outputs: vec![output0],
        };
        let (pk1, _) = make_key(1);
        let witness = Witness::new(tx.id(), &pk1);
        let signed_tx = SignedTransaction {
            tx: tx,
            witnesses: vec![witness.clone()],
        };
        assert_eq!(
            Err(Error::InvalidSignature(utxo0, output0, witness)),
            ledger.diff_transaction(&signed_tx)
        )
    }

    #[test]
    fn cant_loose_money() {
        use chain_core::property::Ledger;
        use chain_core::property::Transaction;
        let (pk1, user1_address) = make_key(0);
        let tx0_id = TransactionId(Hash::hash_bytes(&[0]));
        let utxo0 = UtxoPointer {
            transaction_id: tx0_id,
            output_index: 0,
        };
        let ledger = crate::ledger::Ledger::new(
            vec![(utxo0, Output(user1_address, Value(10)))]
                .iter()
                .cloned()
                .collect(),
        );
        let output0 = Output(user1_address, Value(9));
        let tx = crate::transaction::Transaction {
            inputs: vec![utxo0],
            outputs: vec![output0],
        };
        let witness = Witness::new(tx.id(), &pk1);
        let signed_tx = SignedTransaction {
            tx: tx,
            witnesses: vec![witness],
        };
        assert_eq!(
            Err(Error::TransactionSumIsNonZero(10, 9)),
            ledger.diff_transaction(&signed_tx)
        )
    }

    quickcheck! {
        fn diff_union_is_associative(types: (Diff, Diff, Diff)) -> bool {
            property::testing::update_associativity(types.0, types.1, types.2)
        }
        fn diff_union_has_identity_element(diff: Diff) -> bool {
            property::testing::update_identity_element(diff)
        }
        fn diff_union_has_inverse_element(diff: Diff) -> bool {
            property::testing::update_inverse_element(diff)
        }
        fn diff_union_is_commutative(types: (Diff, Diff)) -> bool {
            property::testing::update_union_commutative(types.0, types.1)
        }
    }

}
