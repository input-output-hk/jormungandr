use imhamt::Hamt;
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
}
