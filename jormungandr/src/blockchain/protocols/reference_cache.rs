use crate::{blockcfg::HeaderHash, blockchain::protocols::Ref};
use std::{collections::HashMap, time::Duration};
use tokio::{
    prelude::*,
    timer::{self, delay_queue, DelayQueue},
};

/// cache of already loaded in-memory block `Ref`
///
pub struct RefCache {
    entries: HashMap<HeaderHash, (Ref, delay_queue::Key)>,
    expirations: DelayQueue<HeaderHash>,

    ttl: Duration,
}

impl RefCache {
    pub fn new(ttl: Duration) -> Self {
        RefCache {
            entries: HashMap::new(),
            expirations: DelayQueue::new(),
            ttl,
        }
    }

    pub fn insert(&mut self, key: HeaderHash, value: Ref) {
        let delay = self.expirations.insert(key.clone(), self.ttl);

        self.entries.insert(key, (value, delay));
    }

    /// accessing the `Ref` will reset the timeout and extend the time
    /// before expiration from the cache.
    pub fn get(&mut self, key: &HeaderHash) -> Option<&Ref> {
        if let Some((v, k)) = self.entries.get(key) {
            self.expirations.reset(k, self.ttl);

            Some(v)
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: &HeaderHash) {
        if let Some((_, cache_key)) = self.entries.remove(key) {
            self.expirations.remove(&cache_key);
        }
    }

    pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
        while let Some(entry) = try_ready!(self.expirations.poll()) {
            self.entries.remove(entry.get_ref());
        }

        Ok(Async::Ready(()))
    }
}
