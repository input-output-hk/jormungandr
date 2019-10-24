use imhamt::Hamt;
use std::collections::hash_map::DefaultHasher;

// Use a Hamt to store a sequence, the indexes can be used for pagination
// XXX: Maybe there is a better data structure for this?
#[derive(Clone)]
pub struct PersistentSequence<T> {
    // Could be usize, but it's only used to store blocks and transactions for now
    len: u32,
    elements: Hamt<DefaultHasher, u32, T>,
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
            elements: self.elements.insert(len - 1, t).unwrap(),
        }
    }

    pub fn get(&self, i: u32) -> Option<&T> {
        self.elements.lookup(&i)
    }

    pub fn len(&self) -> u32 {
        self.len
    }
}
