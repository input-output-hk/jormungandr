use imhamt::{Hamt, HamtIter};
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;

#[derive(Clone)]
pub struct HamtSet<T: Hash + PartialEq + Eq + Clone>(Hamt<DefaultHasher, T, ()>);

impl<T: Hash + PartialEq + Eq + Clone> HamtSet<T> {
    pub fn new() -> HamtSet<T> {
        HamtSet(Hamt::new())
    }

    pub fn add_element(&self, element: T) -> HamtSet<T> {
        let new_hamt = match self.0.insert(element, ()) {
            Ok(new_hamt) => new_hamt,
            Err(_) => self.0.clone(),
        };

        HamtSet(new_hamt)
    }

    pub fn iter(&self) -> HamtSetIter<T> {
        HamtSetIter(self.0.iter())
    }
}

pub struct HamtSetIter<'a, K>(HamtIter<'a, K, ()>);

impl<'a, K> Iterator for HamtSetIter<'a, K> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, _v)| k)
    }
}
