//! Multiverse
//!
//! This is a multi temporal store, where the timeline is accessible by HeaderHash
//! and multiple timelines are possible.
//!
//! For now this only track block at the headerhash level, and doesn't order them
//! temporaly, leaving no way to do garbage collection

use crate::block::HeaderHash;
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
pub struct MultiVerse<ST> {
    known_states: BTreeMap<HeaderHash, ST>,
    tips: BTreeSet<HeaderHash>,
}

impl<ST> MultiVerse<ST> {
    pub fn add(&mut self, prevhash: &HeaderHash, k: &HeaderHash, st: ST) {
        if !self.known_states.contains_key(k) {
            self.known_states.insert(k, st);
            match self.tips.remove(prevhash) {
                None => self.tips.insert(k),
                Some(_) => self.tips.insert(k),
            }
        }
    }

    /// Once the state are old in the timeline, they are less
    /// and less likely to be used anymore, so we leave
    /// a gap between different version that gets bigger and bigger
    pub fn gc(&mut self) {}

    pub fn get(&mut self, k: &HeaderHash) -> Option<ST> {
        self.known_states.get(k)
    }
}
