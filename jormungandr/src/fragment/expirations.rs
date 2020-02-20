use slab::Slab;
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

/// A synchronous alternative to tokio's `DelayQueue` intended to be
/// periodically polled for expired entries.
pub struct Expirations<T> {
    slab: Slab<ExpirationEntry<T>>,
    sorted: BTreeMap<SlotId, HashSet<Key>>,
    first_slot_beginning: Instant,
    // duration of a slot in seconds
    slot_size: u64,
}

struct ExpirationEntry<T> {
    expires_at: SlotId,
    data: T,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct Key(usize);

type SlotId = u64;

impl<T> Expirations<T> {
    pub fn new(slot_size: Duration) -> Self {
        Self {
            slab: Slab::default(),
            sorted: BTreeMap::default(),
            first_slot_beginning: Instant::now(),
            slot_size: slot_size.as_secs(),
        }
    }

    pub fn insert(&mut self, data: T, expires_in: Duration) -> Key {
        let expires_at = self.get_slot_id(Instant::now() + expires_in);
        let entry = ExpirationEntry { expires_at, data };
        let key = Key(self.slab.insert(entry));
        match self.sorted.get_mut(&expires_at) {
            Some(slot) => {
                slot.insert(key);
            }
            None => {
                let mut slot = HashSet::new();
                slot.insert(key);
                self.sorted.insert(expires_at, slot);
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
        let expires_at = self.get_slot_id(Instant::now() + expires_in);
        let mut entry = self.slab.get_mut(key.0).unwrap();
        self.sorted.get_mut(&entry.expires_at).unwrap().remove(&key);
        match self.sorted.get_mut(&expires_at) {
            Some(slot) => {
                slot.insert(key);
            }
            None => {
                let mut slot = HashSet::new();
                slot.insert(key);
                self.sorted.insert(expires_at, slot);
            }
        }
        entry.expires_at = expires_at;
    }

    pub fn pop_expired(&mut self) -> Vec<T> {
        use std::ops::Bound::*;

        let now = self.get_slot_id(Instant::now());
        // the current slot is excluded because it is still in progress
        let range = (Unbounded, Excluded(&now));
        let to_remove = self
            .sorted
            .range(range)
            .map(|(expires_at, entry)| (expires_at.clone(), entry.clone()))
            .collect::<Vec<(SlotId, HashSet<Key>)>>();

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

    fn get_slot_id(&self, time: Instant) -> SlotId {
        (time - self.first_slot_beginning).as_secs() / self.slot_size
    }
}
