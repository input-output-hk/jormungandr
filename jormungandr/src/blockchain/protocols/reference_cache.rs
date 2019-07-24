use crate::{blockcfg::HeaderHash, blockchain::protocols::Ref};
use std::{collections::HashMap, time::Duration};
use tokio::{
    prelude::*,
    timer::{self, delay_queue, DelayQueue},
};

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

    pub fn get(&self, key: &HeaderHash) -> Option<&Ref> {
        self.entries.get(key).map(|&(ref v, _)| v)
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
