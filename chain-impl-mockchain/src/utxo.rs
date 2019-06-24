//! Unspend Transaction Output (UTXO) ledger
//!
//! The UTXO works similarly to cash where the demoninations are of arbitrary values,
//! and each demonination get permanantly consumed by the system once spent.
//!

use crate::transaction::{Output, TransactionId, TransactionIndex};
use std::collections::btree_map;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;

use imhamt::{Hamt, HamtIter, InsertError, RemoveError, ReplaceError, UpdateError};

custom_error! {
    #[derive(Clone, PartialEq, Eq)]
    pub Error
        AlreadyExists = "Transaction ID Already exist",
        TransactionNotFound = "Transaction is not found",
        IndexNotFound = "Index not found",
}

impl From<InsertError> for Error {
    fn from(_: InsertError) -> Error {
        Error::AlreadyExists
    }
}

impl From<UpdateError<()>> for Error {
    fn from(_: UpdateError<()>) -> Error {
        Error::TransactionNotFound
    }
}

impl From<ReplaceError> for Error {
    fn from(_: ReplaceError) -> Error {
        Error::TransactionNotFound
    }
}

impl From<RemoveError> for Error {
    fn from(_: RemoveError) -> Error {
        Error::TransactionNotFound
    }
}

/// Hold all the individual outputs that remain unspent
#[derive(Clone)]
pub struct TransactionUnspents<OutAddress>(BTreeMap<TransactionIndex, Output<OutAddress>>);

impl<OutAddress: Clone> TransactionUnspents<OutAddress> {
    pub fn from_outputs(outs: &[(TransactionIndex, Output<OutAddress>)]) -> Self {
        assert!(outs.len() < 255);
        let mut b = BTreeMap::new();
        for (index, output) in outs.iter() {
            let r = b.insert(*index, output.clone());
            // duplicated index
            if r.is_some() {}
        }
        TransactionUnspents(b)
    }

    pub fn remove_input(
        &self,
        index: TransactionIndex,
    ) -> Result<(Self, Output<OutAddress>), Error> {
        assert!(index < 255);
        let mut t = self.0.clone();
        match t.remove(&index) {
            None => Err(Error::IndexNotFound),
            Some(o) => Ok((TransactionUnspents(t), o)),
        }
    }
}

/// Ledger of UTXO
#[derive(Clone)]
pub struct Ledger<OutAddress>(Hamt<DefaultHasher, TransactionId, TransactionUnspents<OutAddress>>);

pub struct Iter<'a, V> {
    hamt_iter: HamtIter<'a, TransactionId, TransactionUnspents<V>>,
    unspents_iter: Option<(
        &'a TransactionId,
        btree_map::Iter<'a, TransactionIndex, Output<V>>,
    )>,
}

pub struct Values<'a, V> {
    hamt_iter: HamtIter<'a, TransactionId, TransactionUnspents<V>>,
    unspents_iter: Option<btree_map::Iter<'a, TransactionIndex, Output<V>>>,
}

/// structure used by the iterator or the getter of the UTxO `Ledger`
///
pub struct Entry<'a, OutputAddress> {
    pub transaction_id: TransactionId,
    pub output_index: u8,
    pub output: &'a Output<OutputAddress>,
}

impl<OutAddress> Ledger<OutAddress> {
    pub fn iter<'a>(&'a self) -> Iter<'a, OutAddress> {
        Iter {
            hamt_iter: self.0.iter(),
            unspents_iter: None,
        }
    }

    pub fn values<'a>(&'a self) -> Values<'a, OutAddress> {
        Values {
            hamt_iter: self.0.iter(),
            unspents_iter: None,
        }
    }

    pub fn get<'a>(
        &'a self,
        tid: &TransactionId,
        index: &TransactionIndex,
    ) -> Option<Entry<'a, OutAddress>> {
        self.0
            .lookup(tid)
            .and_then(|unspent| unspent.0.get(index))
            .map(|output| Entry {
                transaction_id: tid.clone(),
                output_index: *index,
                output: output,
            })
    }
}

impl<'a, V> Iterator for Values<'a, V> {
    type Item = &'a Output<V>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.unspents_iter {
            None => match self.hamt_iter.next() {
                None => None,
                Some(unspent) => {
                    self.unspents_iter = Some((unspent.1).0.iter());
                    self.next()
                }
            },
            Some(o) => match o.next() {
                None => {
                    self.unspents_iter = None;
                    self.next()
                }
                Some(x) => Some(x.1),
            },
        }
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = Entry<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.unspents_iter {
            None => match self.hamt_iter.next() {
                None => None,
                Some(unspent) => {
                    self.unspents_iter = Some((unspent.0, (unspent.1).0.iter()));
                    self.next()
                }
            },
            Some((id, o)) => match o.next() {
                None => {
                    self.unspents_iter = None;
                    self.next()
                }
                Some(x) => Some(Entry {
                    transaction_id: id.clone(),
                    output_index: *x.0,
                    output: x.1,
                }),
            },
        }
    }
}

impl<OutAddress: Clone> Ledger<OutAddress> {
    /// Create a new empty UTXO Ledger
    pub fn new() -> Self {
        Ledger(Hamt::new())
    }

    /// Add new outputs associated with a specific transaction
    ///
    /// Error if the transaction already exist
    pub fn add(
        &self,
        tid: &TransactionId,
        outs: &[(TransactionIndex, Output<OutAddress>)],
    ) -> Result<Self, Error> {
        assert!(outs.len() < 255);
        let b = TransactionUnspents::from_outputs(outs);
        let next = self.0.insert(tid.clone(), b)?;
        Ok(Ledger(next))
    }

    /// Spend a specific index from the transaction
    ///
    pub fn remove(
        &self,
        tid: &TransactionId,
        index: TransactionIndex,
    ) -> Result<(Self, Output<OutAddress>), Error> {
        let (treemap, output) = match self.0.lookup(tid) {
            None => Err(Error::TransactionNotFound),
            Some(out) => out.remove_input(index),
        }?;

        if treemap.0.len() == 0 {
            Ok((Ledger(self.0.remove(tid)?), output))
        } else {
            Ok((Ledger(self.0.replace(tid, treemap)?.0), output))
        }
    }

    pub fn remove_multiple(
        &self,
        tid: &TransactionId,
        indices: &[TransactionIndex],
    ) -> Result<(Self, Vec<Output<OutAddress>>), Error> {
        let (treemap, outputs) = match self.0.lookup(tid) {
            None => Err(Error::TransactionNotFound),
            Some(out) => {
                let mut treemap = out.clone();
                let mut outputs = Vec::with_capacity(indices.len());
                for index in indices {
                    let (t, o) = treemap.remove_input(*index)?;
                    outputs.push(o);
                    treemap = t;
                }
                Ok((treemap, outputs))
            }
        }?;

        if treemap.0.len() == 0 {
            Ok((Ledger(self.0.remove(tid)?), outputs))
        } else {
            Ok((Ledger(self.0.replace(tid, treemap)?.0), outputs))
        }
    }
}
