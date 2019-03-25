//! Multiverse
//!
//! This is a multi temporal store, where the timeline is accessible by HeaderHash
//! and multiple timelines are possible.
//!
//! For now this only track block at the headerhash level, and doesn't order them
//! temporaly, leaving no way to do garbage collection

use chain_core::property::BlockId;
use std::collections::{hash_map::Entry, BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

//
// The multiverse is characterized by a single origin and multiple state of a given time
//
//          [root A]
//        ,o            ,-o-o--o [root B]
//       /             /
// o----o----o--o--o--o-o-o-o-oooo [root E]
//                  \
//                   `-o--o [root C]
//                      \
//                      `----o-o-oo [root F]
//
// +------------------------------+-----> time
// t=0                            t=latest known
//
pub struct Multiverse<Hash: BlockId, ST> {
    known_states: BTreeMap<Hash, ST>,
    roots: Arc<RwLock<Roots<Hash>>>,
}

struct Roots<Hash: BlockId> {
    roots: HashMap<Hash, usize>,
}

/// A RAII wrapper around a block identifier that keeps the state
/// corresponding to the block pinned in memory.
pub struct GCRoot<Hash: BlockId> {
    hash: Hash,
    roots: Arc<RwLock<Roots<Hash>>>,
}

impl<Hash: BlockId> GCRoot<Hash> {
    fn new(hash: Hash, roots: Arc<RwLock<Roots<Hash>>>) -> Self {
        {
            let mut roots = roots.write().unwrap();
            *roots.roots.entry(hash.clone()).or_insert(0) += 1;
        }

        GCRoot { hash, roots }
    }
}

impl<Hash: BlockId> std::ops::Deref for GCRoot<Hash> {
    type Target = Hash;
    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

impl<Hash: BlockId> Drop for GCRoot<Hash> {
    fn drop(&mut self) {
        let mut roots = self.roots.write().unwrap();
        if let Entry::Occupied(mut entry) = roots.roots.entry(self.hash.clone()) {
            if *entry.get() > 1 {
                *entry.get_mut() -= 1;
            } else {
                //println!("state for block {:?} became garbage", self.hash);
                entry.remove_entry();
                // put on GC list?
            }
        } else {
            unreachable!();
        }
    }
}

impl<Hash: BlockId, ST> Multiverse<Hash, ST> {
    pub fn new() -> Self {
        Multiverse {
            known_states: BTreeMap::new(),
            roots: Arc::new(RwLock::new(Roots {
                roots: HashMap::new(),
            })),
        }
    }

    /// Add a state to the multiverse. Return a GCRoot object that
    /// pins the state into memory.
    pub fn add(&mut self, k: Hash, st: ST) -> GCRoot<Hash> {
        self.known_states.entry(k.clone()).or_insert(st);

        GCRoot::new(k, self.roots.clone())
    }

    /// Once the state are old in the timeline, they are less
    /// and less likely to be used anymore, so we leave
    /// a gap between different version that gets bigger and bigger
    pub fn gc(&mut self) {
        let roots = self.roots.read().unwrap();

        let mut garbage = vec![];

        for (k, _) in &self.known_states {
            if !roots.roots.contains_key(&k) {
                garbage.push(k.clone());
            }
        }

        println!("deleting {} states from multiverse", garbage.len());

        for k in garbage {
            //println!("deleting state {:?}", k);
            self.known_states.remove(&k);
        }
    }

    pub fn get(&self, k: &Hash) -> Option<&ST> {
        self.known_states.get(&k)
    }

    pub fn get_from_root(&self, root: &GCRoot<Hash>) -> &ST {
        assert!(Arc::ptr_eq(&root.roots, &self.roots));
        self.get(&*root).unwrap()
    }
}
