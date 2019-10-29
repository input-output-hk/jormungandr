//! Unspend Transaction Output (UTXO) ledger
//!
//! The UTXO works similarly to cash where the demoninations are of arbitrary values,
//! and each demonination get permanantly consumed by the system once spent.
//!

use crate::fragment::FragmentId;
use crate::transaction::{Output, TransactionIndex};
use chain_addr::Address;
use std::collections::btree_map;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::fmt;

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
#[derive(Clone, PartialEq, Eq, Debug)]
struct TransactionUnspents<OutAddress>(BTreeMap<TransactionIndex, Output<OutAddress>>);

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
#[derive(Clone, PartialEq, Eq)]
pub struct Ledger<OutAddress>(Hamt<DefaultHasher, FragmentId, TransactionUnspents<OutAddress>>);

pub struct Iter<'a, V> {
    hamt_iter: HamtIter<'a, FragmentId, TransactionUnspents<V>>,
    unspents_iter: Option<(
        &'a FragmentId,
        btree_map::Iter<'a, TransactionIndex, Output<V>>,
    )>,
}

pub struct Values<'a, V> {
    hamt_iter: HamtIter<'a, FragmentId, TransactionUnspents<V>>,
    unspents_iter: Option<btree_map::Iter<'a, TransactionIndex, Output<V>>>,
}

impl fmt::Debug for Ledger<Address> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// structure used by the iterator or the getter of the UTxO `Ledger`
///
#[derive(Debug, PartialEq, Clone)]
pub struct Entry<'a, OutputAddress> {
    pub fragment_id: FragmentId,
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
        tid: &FragmentId,
        index: &TransactionIndex,
    ) -> Option<Entry<'a, OutAddress>> {
        self.0
            .lookup(tid)
            .and_then(|unspent| unspent.0.get(index))
            .map(|output| Entry {
                fragment_id: tid.clone(),
                output_index: *index,
                output: output,
            })
    }
}

impl<'a, V> Iterator for Values<'a, V> {
    type Item = &'a Output<V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.unspents_iter {
                None => match self.hamt_iter.next() {
                    None => return None,
                    Some(unspent) => self.unspents_iter = Some((unspent.1).0.iter()),
                },
                Some(o) => match o.next() {
                    None => self.unspents_iter = None,
                    Some(x) => return Some(x.1),
                },
            }
        }
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = Entry<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.unspents_iter {
                None => match self.hamt_iter.next() {
                    None => return None,
                    Some(unspent) => self.unspents_iter = Some((unspent.0, (unspent.1).0.iter())),
                },
                Some((id, o)) => match o.next() {
                    None => self.unspents_iter = None,
                    Some(x) => {
                        return Some(Entry {
                            fragment_id: id.clone(),
                            output_index: *x.0,
                            output: x.1,
                        })
                    }
                },
            }
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
        tid: &FragmentId,
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
        tid: &FragmentId,
        index: TransactionIndex,
    ) -> Result<(Self, Output<OutAddress>), Error> {
        let (treemap, output) = match self.0.lookup(tid) {
            None => Err(Error::TransactionNotFound),
            Some(out) => out.remove_input(index),
        }?;

        if treemap.0.is_empty() {
            Ok((Ledger(self.0.remove(tid)?), output))
        } else {
            Ok((Ledger(self.0.replace(tid, treemap)?.0), output))
        }
    }

    pub fn remove_multiple(
        &self,
        tid: &FragmentId,
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

        if treemap.0.is_empty() {
            Ok((Ledger(self.0.remove(tid)?), outputs))
        } else {
            Ok((Ledger(self.0.replace(tid, treemap)?.0), outputs))
        }
    }
}

impl<OutAddress: Clone>
    std::iter::FromIterator<(FragmentId, Vec<(TransactionIndex, Output<OutAddress>)>)>
    for Ledger<OutAddress>
{
    fn from_iter<
        I: IntoIterator<Item = (FragmentId, Vec<(TransactionIndex, Output<OutAddress>)>)>,
    >(
        iter: I,
    ) -> Self {
        let mut ledger = Ledger::new();
        for (tid, outputs) in iter {
            ledger = ledger.add(&tid, &outputs).unwrap();
        }
        ledger
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        key::Hash, testing::arbitrary::AverageValue, testing::data::AddressData, testing::TestGen,
        value::Value,
    };
    use chain_addr::{Address, Discrimination};
    use quickcheck::{Arbitrary, Gen, TestResult};
    use quickcheck_macros::quickcheck;
    use std::collections::HashMap;
    use std::iter;

    #[derive(Clone, Debug)]
    pub struct ArbitraryUtxos(HashMap<FragmentId, ArbitraryTransactionOutputs>);

    impl Arbitrary for ArbitraryUtxos {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = usize::arbitrary(g) % 10 + 1;
            let collection: HashMap<FragmentId, ArbitraryTransactionOutputs> =
                iter::from_fn(|| {
                    Some((
                        Hash::arbitrary(g),
                        ArbitraryTransactionOutputs::arbitrary(g),
                    ))
                })
                .take(size)
                .collect();

            ArbitraryUtxos(collection)
        }
    }

    impl ArbitraryUtxos {
        pub fn fill(&self, mut ledger: Ledger<Address>) -> Ledger<Address> {
            for (key, value) in self.0.iter() {
                let utxo = value.to_vec();
                ledger = ledger.add(&key, &utxo.as_slice()).unwrap();
            }
            ledger
        }
    }

    #[derive(Debug, Clone)]
    pub struct ArbitraryTransactionOutputs {
        pub utxos: HashMap<TransactionIndex, Output<Address>>,
        pub idx_to_remove: TransactionIndex,
    }

    impl Arbitrary for ArbitraryTransactionOutputs {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let size = usize::arbitrary(g) % 50 + 1;
            let iter = iter::from_fn(|| {
                Some(AddressData::arbitrary(g).make_output(AverageValue::arbitrary(g).into()))
            })
            .enumerate()
            .take(size);

            let mut outputs = HashMap::new();
            for (i, item) in iter {
                outputs.insert(i as u8, item.clone());
            }

            let idx_to_remove = usize::arbitrary(g);

            ArbitraryTransactionOutputs {
                utxos: outputs,
                idx_to_remove: idx_to_remove as u8,
            }
        }
    }

    impl ArbitraryTransactionOutputs {
        pub fn to_vec(&self) -> Vec<(TransactionIndex, Output<Address>)> {
            let mut outputs = Vec::with_capacity(self.utxos.len());
            for (key, value) in self.utxos.iter() {
                outputs.push((*key as u8, value.clone()));
            }
            outputs
        }
    }

    impl Arbitrary for Ledger<Address> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut ledger = Ledger::new();
            let arbitrary_utxos = ArbitraryUtxos::arbitrary(g);

            for (key, value) in arbitrary_utxos.0 {
                let utxo = value.to_vec();
                ledger = ledger.add(&key, &utxo.as_slice()).unwrap();
            }
            ledger
        }
    }

    #[quickcheck]
    pub fn transaction_unspent_remove(outputs: ArbitraryTransactionOutputs) -> TestResult {
        let transaction_unspent = TransactionUnspents::from_outputs(outputs.to_vec().as_slice());
        let is_index_correct = transaction_unspent.0.contains_key(&outputs.idx_to_remove);
        match (
            transaction_unspent.remove_input(outputs.idx_to_remove),
            is_index_correct,
        ) {
            (Ok((transaction_unspent, output)), true) => {
                if transaction_unspent.0.contains_key(&outputs.idx_to_remove) {
                    return TestResult::error("Element not removed");
                }

                assert_eq!(
                    output.clone(),
                    outputs.utxos.get(&outputs.idx_to_remove).unwrap().clone(),
                    "Outputs are different"
                );
                TestResult::passed()
            }
            (Ok(_), false) => TestResult::error("Element removed, while it should not"),
            (Err(err), true) => TestResult::error(format!("Unexpected error {}", err)),
            (Err(_), false) => TestResult::passed(),
        }
    }

    #[quickcheck]
    pub fn ledger_iter_values_correctly(initial_utxos: ArbitraryUtxos) -> TestResult {
        let mut ledger = Ledger::new();
        ledger = initial_utxos.fill(ledger);

        // use iter
        for (key, value) in initial_utxos.0 {
            for (id, output) in value.to_vec() {
                if !ledger.iter().any(|x| {
                    x.fragment_id == key && x.output_index == id && x.output.clone() == output
                }) {
                    return TestResult::error(format!(
                        "Cannot find item using iter: {:?},{:?}",
                        key, id
                    ));
                }
            }
        }
        TestResult::passed()
    }

    #[test]
    pub fn remove_outputs_from_ledger() {
        let mut ledger = Ledger::new();
        let first_fragment_id = TestGen::hash();
        let second_fragment_id = TestGen::hash();
        let first_address_data = AddressData::utxo(Discrimination::Test).make_output(Value(100));
        let second_address_data = AddressData::utxo(Discrimination::Test).make_output(Value(100));
        let first_index = 0 as u8;
        let second_index = 1 as u8;

        ledger = ledger
            .add(
                &first_fragment_id,
                &[
                    (first_index, first_address_data.clone()),
                    (second_index, second_address_data.clone()),
                ],
            )
            .expect("Unable to add first output");

        ledger = ledger
            .add(
                &second_fragment_id,
                &[
                    (first_index, first_address_data.clone()),
                    (second_index, second_address_data.clone()),
                ],
            )
            .expect("Unable to add second output");

        // remove single output
        let (ledger, output_address) = ledger
            .remove(&first_fragment_id, first_index)
            .expect("Unable to remove single output (first output)");
        assert_eq!(output_address, first_address_data.clone());
        assert!(ledger.get(&first_fragment_id, &first_index).is_none());
        assert!(ledger.get(&first_fragment_id, &second_index).is_some());
        assert_eq!(ledger.iter().count(), 3);

        //remove single output, which is last output in fragment. This should lead to removal of fragment
        let (ledger, output_address) = ledger
            .remove(&first_fragment_id, second_index)
            .expect("Unable to remove single output (second output)");
        assert_eq!(output_address, second_address_data.clone());
        assert!(ledger.get(&first_fragment_id, &second_index).is_none());

        assert_eq!(ledger.iter().count(), 2);

        // remove multiple outputs
        let (ledger, output_addresses) = ledger
            .remove_multiple(&second_fragment_id, &[first_index, second_index])
            .expect("Unable to remove multiple output");

        let expected_output_addresses =
            vec![first_address_data.clone(), second_address_data.clone()];
        assert_eq!(output_addresses, expected_output_addresses);
        assert_eq!(ledger.iter().count(), 0);
    }
}
