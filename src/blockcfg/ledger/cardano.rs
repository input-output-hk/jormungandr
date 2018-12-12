use std::collections::{BTreeMap};

use crate::blockcfg::ledger;
use crate::blockcfg::chain::cardano::{Transaction};

use cardano::{
    block::{
        verify_chain::{ChainState},
        verify::{Error},
    },
    tx::{TxoPointer, TxOut},
};

#[derive(Debug, Clone)]
pub struct Diff {
    spent_outputs: BTreeMap<TxoPointer, TxOut>,
    new_unspent_outputs: BTreeMap<TxoPointer, TxOut>,
}
impl Diff {
    fn new() -> Self {
        Diff {
            spent_outputs: BTreeMap::new(),
            new_unspent_outputs: BTreeMap::new(),
        }
    }

    fn extend(&mut self, other: Self) {
        self.new_unspent_outputs.extend(other.new_unspent_outputs);
        self.spent_outputs.extend(other.spent_outputs);
    }
}

impl ledger::Ledger for ChainState {
    type Transaction = Transaction;
    type Diff = Diff;
    type Error = Error;


    fn diff_transaction(&self, transaction: &Self::Transaction) -> Result<Self::Diff, Self::Error> {
        use cardano::block::verify::{Verify};

        let id = transaction.tx.id();
        let mut diff = Diff::new();

        // 1. verify the transaction is valid (self valid)
        transaction.verify(self.protocol_magic)?;

        for (input, witness) in transaction.tx.inputs.iter().zip(transaction.witness.iter()) {
            if let Some(output) = self.utxos.get(&input) {
                if ! witness.verify_address(&output.address) {
                    return Err(Error::AddressMismatch);
                }
                if let Some(_output) = diff.spent_outputs.insert(input.clone(), output.clone()) {
                    return Err(Error::DuplicateInputs);
                }

            } else {
                return Err(Error::MissingUtxo);
            }
        }

        // 2. prepare to add the new outputs
        for (index, output) in transaction.tx.outputs.iter().enumerate() {
            diff.new_unspent_outputs.insert(
                TxoPointer::new(id, index as u32),
                output.clone()
            );
        }

        Ok(diff)
    }
    fn diff<'a, I>(&self, transactions: I) -> Result<Self::Diff, Self::Error>
        where I: Iterator<Item = &'a Self::Transaction> + Sized
            , Self::Transaction: 'a
    {
        let mut diff = Diff::new();

        for transaction in transactions {
            diff.extend(self.diff_transaction(transaction)?);
        }

        Ok(diff)
    }
    fn add(&mut self, diff: Self::Diff) -> Result<&mut Self, Self::Error>
    {
        for spent_output in diff.spent_outputs.keys() {
            if let None = self.utxos.remove(spent_output) {
                return Err(Error::MissingUtxo);
            }
        }

        for (input, output) in diff.new_unspent_outputs {
            if let Some(_original_output) = self.utxos.insert(input, output) {
                return Err(Error::DuplicateTxo);
            }
        }

        Ok(self)
    }
}
