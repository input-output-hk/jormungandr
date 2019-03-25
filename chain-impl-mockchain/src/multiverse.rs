//! Multiverse
//!
//! This is a multi temporal store, where the timeline is accessible by HeaderHash
//! and multiple timelines are possible.
//!
//! For now this only track block at the headerhash level, and doesn't order them
//! temporaly, leaving no way to do garbage collection

use chain_core::property::BlockId;
use std::collections::{BTreeMap, BTreeSet};

//
// The multiverse is characterized by a single origin and multiple state of a given time
//
//          [tip A]
//        ,o            ,-o-o--o [tip B]
//       /             /
// o----o----o--o--o--o-o-o-o-oooo [tip E]
//                  \
//                   `-o--o [tip C]
//                      \
//                      `----o-o-oo [tip F]
//
// +------------------------------+-----> time
// t=0                            t=latest known
//
pub struct Multiverse<Hash: BlockId, ST> {
    known_states: BTreeMap<Hash, ST>,
    tips: BTreeSet<Hash>,
}

impl<Hash: BlockId, ST> Multiverse<Hash, ST> {
    pub fn new() -> Self {
        Multiverse {
            known_states: BTreeMap::new(),
            tips: BTreeSet::new(),
        }
    }

    pub fn add(&mut self, k: &Hash, st: ST) {
        if !self.known_states.contains_key(k) {
            self.known_states.insert(k.clone(), st);
            /*
            self.tips.remove(prevhash);
            self.tips.insert(k.clone());
            */
        }
    }

    /// Once the state are old in the timeline, they are less
    /// and less likely to be used anymore, so we leave
    /// a gap between different version that gets bigger and bigger
    pub fn gc(&mut self) {}

    pub fn get(&self, k: &Hash) -> Option<&ST> {
        self.known_states.get(&k)
    }
}
