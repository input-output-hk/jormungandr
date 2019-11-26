use imhamt::Hamt;
use std::{collections::hash_map::DefaultHasher, sync::Arc};

// Use a Hamt to store a sequence, the indexes can be used for pagination
// XXX: Maybe there is a better data structure for this?
#[derive(Clone)]
pub struct PersistentSequence<T> {
    len: u64,
    elements: Hamt<DefaultHasher, u64, Arc<T>>,
}

impl<T> PersistentSequence<T> {
    pub fn new() -> Self {
        PersistentSequence {
            len: 0,
            elements: Hamt::new(),
        }
    }

    pub fn append(&self, t: T) -> Self {
        let len = self.len + 1;
        PersistentSequence {
            len,
            elements: self.elements.insert(len - 1, Arc::new(t)).unwrap(),
        }
    }

    pub fn get<I: Into<u64>>(&self, i: I) -> Option<&Arc<T>> {
        self.elements.lookup(&i.into())
    }

    pub fn len(&self) -> u64 {
        self.len
    }
}
