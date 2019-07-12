use crate::{
    crypto::hash::Hash,
    interfaces::{Address, Value},
};
use chain_impl_mockchain::utxo::Entry;
use serde::{Deserialize, Serialize};

/// the Unspent Transaction Output information.
///
/// This object contains all the information we know about a [UTxO].
/// This data is different from the [`AccountState`] which represents
/// the state of an account in the ledger.
///
/// [UTxO]: https://en.wikipedia.org/wiki/Unspent_transaction_output
/// [`AccountState`]: ./struct.AccountState.html
///
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct UTxOInfo {
    transaction_id: Hash,
    index_in_transaction: u8,
    address: Address,
    associated_fund: Value,
}

impl UTxOInfo {
    /// the Transaction identifier (its hash) that will be used to reference
    /// to this UTxO as an input in a new transaction.
    ///
    /// Along with the `index_in_transaction` this uniquely identifies an UTxO
    #[inline]
    pub fn transaction_id(&self) -> &Hash {
        &self.transaction_id
    }

    /// the output index, will be needed as an input in a new transaction.
    ///
    /// Along with the `transaction_id` this uniquely identifies an UTxO
    #[inline]
    pub fn index_in_transaction(&self) -> u8 {
        self.index_in_transaction
    }

    /// the address to identify who can spend the UTxO. This is part of the
    /// data actually present as output of the source transaction.
    #[inline]
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// the associated funds in this UTxO. This will be used in a new transaction
    /// input to guarantee self validation of the transaction's balance.
    #[inline]
    pub fn associated_fund(&self) -> &Value {
        &self.associated_fund
    }
}

/* ---------------- Conversion --------------------------------------------- */

impl<'a> From<Entry<'a, chain_addr::Address>> for UTxOInfo {
    fn from(utxo_entry: Entry<'a, chain_addr::Address>) -> Self {
        Self {
            transaction_id: utxo_entry.fragment_id.into(),
            index_in_transaction: utxo_entry.output_index,
            address: utxo_entry.output.address.clone().into(),
            associated_fund: utxo_entry.output.value.into(),
        }
    }
}
