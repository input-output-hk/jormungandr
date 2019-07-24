use crate::{blockcfg::HeaderHash, blockchain::protocols::Ref};
use std::{collections::HashMap, convert::Infallible, time::Duration};
use tokio::{
    prelude::*,
    sync::lock::Lock,
    timer::{self, delay_queue, DelayQueue},
};

#[derive(Clone)]
pub struct RefCache {
    inner: Lock<RefCacheData>,
}

/// cache of already loaded in-memory block `Ref`
///
struct RefCacheData {
    entries: HashMap<HeaderHash, (Ref, delay_queue::Key)>,
    expirations: DelayQueue<HeaderHash>,

    ttl: Duration,
}

impl RefCache {
    pub fn new(ttl: Duration) -> Self {
        RefCache {
            inner: Lock::new(RefCacheData::new(ttl)),
        }
    }

    pub fn insert(
        &self,
        key: HeaderHash,
        value: Ref,
    ) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.insert(key, value))
    }

    pub fn get(&self, key: HeaderHash) -> impl Future<Item = Option<Ref>, Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.get(&key).cloned())
    }

    pub fn remove(&self, key: HeaderHash) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).map(move |mut guard| guard.remove(&key))
    }

    pub fn purge(&self) -> impl Future<Item = (), Error = timer::Error> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(|mut guard| future::poll_fn(move || guard.poll_purge()))
    }
}

impl RefCacheData {
    fn new(ttl: Duration) -> Self {
        RefCacheData {
            entries: HashMap::new(),
            expirations: DelayQueue::new(),
            ttl,
        }
    }

    fn insert(&mut self, key: HeaderHash, value: Ref) {
        let delay = self.expirations.insert(key.clone(), self.ttl);

        self.entries.insert(key, (value, delay));
    }

    /// accessing the `Ref` will reset the timeout and extend the time
    /// before expiration from the cache.
    fn get(&mut self, key: &HeaderHash) -> Option<&Ref> {
        if let Some((v, k)) = self.entries.get(key) {
            self.expirations.reset(k, self.ttl);

            Some(v)
        } else {
            None
        }
    }

    fn remove(&mut self, key: &HeaderHash) {
        if let Some((_, cache_key)) = self.entries.remove(key) {
            self.expirations.remove(&cache_key);
        }
    }

    fn poll_purge(&mut self) -> Poll<(), timer::Error> {
        while let Some(entry) = try_ready!(self.expirations.poll()) {
            self.entries.remove(entry.get_ref());
        }

        Ok(Async::Ready(()))
    }
}
