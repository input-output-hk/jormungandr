use slab::Slab;
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

/// A synchronous alternative to tokio's `DelayQueue` intended to be
/// periodically polled for expired entries.
pub struct Expirations<T> {
    slab: Slab<ExpirationEntry<T>>,
    sorted: BTreeMap<Instant, HashSet<Key>>,
}

struct ExpirationEntry<T> {
    expires_at: Instant,
    data: T,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key(usize);

impl<T> Expirations<T> {
    pub fn new() -> Self {
        Self {
            slab: Slab::default(),
            sorted: BTreeMap::default(),
        }
    }

    pub fn insert(&mut self, data: T, expires_in: Duration) -> Key {
        let expires_at = Instant::now() + expires_in;
        let entry = ExpirationEntry { expires_at, data };
        let key = Key(self.slab.insert(entry));
        match self.sorted.get_mut(&expires_at) {
            Some(chunk) => {
                chunk.insert(key);
            }
            None => {
                let mut hs = HashSet::new();
                hs.insert(key);
                self.sorted.insert(expires_at, hs);
            }
        }
        key
    }

    pub fn remove(&mut self, key: Key) -> T {
        let entry = self.slab.remove(key.0);
        self.sorted.remove(&entry.expires_at);
        entry.data
    }

    pub fn reschedule(&mut self, key: Key, expires_in: Duration) {
        let expires_at = Instant::now() + expires_in;
        let mut entry = self.slab.get_mut(key.0).unwrap();
        self.sorted.get_mut(&entry.expires_at).unwrap().remove(&key);
        match self.sorted.get_mut(&expires_at) {
            Some(chunk) => {
                chunk.insert(key);
            }
            None => {
                let mut hs = HashSet::new();
                hs.insert(key);
                self.sorted.insert(expires_at, hs);
            }
        }
        entry.expires_at = expires_at;
    }

    pub fn pop_expired(&mut self) -> Vec<T> {
        use std::ops::Bound::*;
        let now = Instant::now();
        let to_remove = self
            .sorted
            .range((Unbounded, Included(&now)))
            .map(|(expires_at, entry)| (expires_at.clone(), entry.clone()))
            .collect::<Vec<(Instant, HashSet<Key>)>>();

        for (expires_at, _) in to_remove.iter() {
            self.sorted.remove(expires_at);
        }

        to_remove
            .into_iter()
            .map(|(_, slab_keys)| slab_keys.into_iter())
            .flatten()
            .map(|key| self.slab.remove(key.0).data)
            .collect()
    }
}
