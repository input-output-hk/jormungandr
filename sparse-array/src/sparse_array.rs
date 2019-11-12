use crate::bitmap::BitmapIndex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SparseArray<V> {
    index: BitmapIndex,
    data: Vec<V>,
}

impl<V> SparseArray<V> {
    pub fn new() -> Self {
        Self {
            index: BitmapIndex::new(),
            data: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: u8) -> Self {
        Self {
            index: BitmapIndex::new(),
            data: Vec::with_capacity(capacity as usize),
        }
    }

    pub fn get(&self, idx: u8) -> Option<&V> {
        self.index
            .get_real_index(idx)
            .map(|idx_real| self.data.get(idx_real as usize).unwrap())
    }

    pub fn set(&mut self, idx: u8, value: V) {
        match self.index.get_real_index(idx) {
            Some(idx_real) => self.data[idx_real as usize] = value,
            None => {
                self.index.set_index(idx);
                let idx_real = self.index.get_real_index(idx).unwrap();
                self.data.insert(idx_real as usize, value);
            }
        }
    }

    pub fn remove(&mut self, idx: u8) -> Option<V> {
        let r = self
            .index
            .get_real_index(idx)
            .map(|idx_real| self.data.remove(idx_real as usize));
        self.index.remove_index(idx);
        r
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn contains_key(&self, idx: u8) -> bool {
        self.index.get_index(idx)
    }

    pub fn iter(&self) -> SparseArrayIter<V> {
        SparseArrayIter::new(&self)
    }
}

pub struct SparseArrayIter<'a, V> {
    bitmap: BitmapIndex,
    sparse_array: &'a SparseArray<V>,
}

impl<'a, V> SparseArrayIter<'a, V> {
    pub fn new(sparse_array: &'a SparseArray<V>) -> Self {
        Self {
            bitmap: sparse_array.index.clone(),
            sparse_array,
        }
    }
}

impl<'a, V> Iterator for SparseArrayIter<'a, V> {
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

        let mut sparse_array = SparseArray::new();
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

        let mut sparse_array = SparseArray::new();
        for (idx, value) in data.iter() {
            sparse_array.set(*idx, value);
        }

        let (to_remove, to_set) = data.split_at(data.len() / 2);
        for (idx, _) in to_remove.iter() {
            sparse_array.remove(*idx);
        }

        to_remove
            .iter()
            .all(|(idx, _)| sparse_array.get(*idx) == None)
            && to_set
                .iter()
                .all(|(idx, value)| sparse_array.get(*idx) == Some(&value))
    }
}
