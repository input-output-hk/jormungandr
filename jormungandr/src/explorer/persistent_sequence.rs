use imhamt::Hamt;
use std::convert::Infallible;
use std::{collections::hash_map::DefaultHasher, sync::Arc};

/// Use a Hamt to store a sequence, the indexes can be used for pagination

// TODO:
// this data structure may be better served by either
//   a persistent linked list (although pagination will be suffer)
//   a persistent btree
//   a persistent prefix tree (which would be like the hamt, I think, but without
//   hashing the keys before, so we don't lose locality)
// but it is used by different things now, and maybe some would benefit more
// from one of the options than the others

#[derive(Clone)]
pub struct PersistentSequence<T> {
    len: u64,
    elements: Hamt<DefaultHasher, u64, Arc<T>>,
    /// this is the first valid index, as the sequence doesn't need to start from
    /// 0, we need this to remove from the beginning, which is useful to undo
    /// blocks, because we are always undoing blocks from the back sequentially,
    /// in the opposite order they were applied, so we never need to remove from
    /// the beginning
    first: Option<u64>,
}

impl<T> PersistentSequence<T> {
    pub fn new() -> Self {
        PersistentSequence {
            len: 0,
            elements: Hamt::new(),
            first: None,
        }
    }

    pub fn append(&self, t: T) -> Self {
        let len = self.len + 1;
        let first = self.first.or_else(|| Some(0)).map(|first| first + 1);

        PersistentSequence {
            len,
            elements: self.elements.insert(len - 1, Arc::new(t)).unwrap(),
            first,
        }
    }

    pub fn remove_first(&self) -> Option<(Self, Arc<T>)> {
        self.first.and_then(|first| {
            let mut deleted = None;
            let elements = self
                .elements
                .update::<_, Infallible>(&first, |elem| Ok(deleted.replace(Arc::clone(elem))))
                .ok()?;

            deleted.map(|deleted| {
                (
                    PersistentSequence {
                        elements,
                        len: self.len - 1,
                        first: Some(first + 1),
                    },
                    deleted,
                )
            })
        })
    }

    pub fn get<I: Into<u64>>(&self, i: I) -> Option<&Arc<T>> {
        self.elements.lookup(&i.into())
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}

impl<T> Default for PersistentSequence<T> {
    fn default() -> Self {
        PersistentSequence::new()
    }
}
