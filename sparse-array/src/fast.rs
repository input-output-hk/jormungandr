///! Wrapper for sparse arrays that doesn't delete anything from the memory
/// unless `shrink` is called.

use std::sync::Arc;
use crate::{
    bitmap::BitmapIndex,
    SparseArray,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FastSparseArray<V> {
    index: BitmapIndex,
    data: Arc<SparseArray<V>>,
}

impl<V: Clone> FastSparseArray<V> {
    pub fn new() -> Self {
        Self {
            index: BitmapIndex::new(),
            data: Arc::new(SparseArray::new()),
        }
    }

    pub fn with_capacity(capacity: u8) -> Self {
        Self {
            index: BitmapIndex::new(),
            data: Arc::new(SparseArray::with_capacity(capacity)),
        }
    }

    pub fn get(&self, idx: u8) -> Option<&V> {
        if !self.index.get_index(idx) {
            None
        } else {
            self.data.get(idx)
        }
    }

    pub fn set(&mut self, idx: u8, value: V) {
        match Arc::get_mut(&mut self.data) {
            Some(d) => d.set(idx, value),
            None => {
                let mut new_sparse_array = SparseArray::new();
                for (idx, value) in self.iter() {
                    new_sparse_array.set(idx, (*value).clone());
                }
                new_sparse_array.set(idx, value);
                self.data = Arc::new(new_sparse_array);
            }
        }
        self.index.set_index(idx);
    }

    pub fn remove(&mut self, idx: u8) -> Option<V> {
        if self.index.get_index(idx) {
            self.index.remove_index(idx);
            return self.data.get(idx).map(|x| (*x).clone());
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn contains_key(&self, idx: u8) -> bool {
        self.index.get_index(idx)
    }

    pub fn iter(&self) -> FastSparseArrayIter<V> {
        FastSparseArrayIter::new(&self)
    }

    pub fn shrink(&mut self) {
        let mut new_sparse_array = SparseArray::new();
        for (idx, value) in self.iter() {
            new_sparse_array.set(idx, (*value).clone());
        }
        self.data = Arc::new(new_sparse_array);
    }
}

pub struct FastSparseArrayIter<'a, V> {
    bitmap: BitmapIndex,
    sparse_array: &'a FastSparseArray<V>,
}

impl<'a, V> FastSparseArrayIter<'a, V> {
    pub fn new(sparse_array: &'a FastSparseArray<V>) -> Self {
        Self {
            bitmap: sparse_array.index.clone(),
            sparse_array,
        }
    }
}

impl<'a, V: Clone> Iterator for FastSparseArrayIter<'a, V> {
    type Item = (u8, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        match self.bitmap.get_first_index() {
            Some(idx) => {
                self.bitmap.remove_index(idx);
                Some((idx, self.sparse_array.get(idx).unwrap()))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[quickcheck]
    fn add_test(data: Vec<(u8, u8)>) -> bool {
        let mut data = data;
        data.sort_by(|a, b| a.0.cmp(&b.0));
        data.dedup_by(|a, b| a.0.eq(&b.0));

        let mut sparse_array = FastSparseArray::new();
        for (idx, value) in data.iter() {
            sparse_array.set(*idx, value);
        }

        data.iter()
            .all(|(idx, value)| sparse_array.get(*idx) == Some(&value))
    }

    #[quickcheck]
    fn remove_test(data: Vec<(u8, u8)>) -> bool {
        let mut data = data;
        data.sort_by(|a, b| a.0.cmp(&b.0));
        data.dedup_by(|a, b| a.0.eq(&b.0));

        let mut sparse_array = FastSparseArray::new();
        for (idx, value) in data.iter() {
            sparse_array.set(*idx, value);
        }

        let (to_remove, to_set) = data.split_at(data.len() / 2);
        for (idx, _) in to_remove.iter() {
            sparse_array.remove(*idx);
        }

        sparse_array.shrink();

        to_remove
            .iter()
            .all(|(idx, _)| sparse_array.get(*idx) == None)
            && to_set
                .iter()
                .all(|(idx, value)| sparse_array.get(*idx) == Some(&value))
    }

    #[test]
    fn test_original_copy_not_changed_add() {
        let mut sparse_array = FastSparseArray::new();
        
        let original_value = 1;
        let new_value = 2;
        let original_idx = 10;
        let new_idx = 15;

        sparse_array.set(original_idx, original_value);
        
        let mut new_array = sparse_array.clone();
        new_array.set(new_idx, new_value);

        assert_eq!(sparse_array.get(new_idx), None);
    }

    #[test]
    fn test_original_copy_not_changed_remove() {
        let mut sparse_array = FastSparseArray::new();
        
        let original_value = 1;
        let original_idx = 2;
        sparse_array.set(original_idx, original_value);
        
        let mut new_array = sparse_array.clone();
        new_array.remove(original_idx);

        assert_eq!(sparse_array.get(original_idx), Some(&original_value));
    }
}
