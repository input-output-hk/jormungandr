use crate::blockcfg::HeaderHash;
use std::{collections::HashMap, convert::Infallible, time::Duration};
use tokio::{
    prelude::*,
    sync::lock::Lock,
    timer::{self, delay_queue, DelayQueue},
};

/// Cache for temporary block data such as [`Ref`]. Every time an entry
/// is accessed its TTL will be reset. Once the TTL of the entry has
/// expired it may be removed from the cache.
///
/// The cache expired entry will be removed only if the entry's
/// TTL has expired and [`purge`] has been called and has completed.
///
/// [`Ref`]: ./struct.Ref.html
/// [`purge`]: ./struct.BlockCache.html#method.purge
#[derive(Clone)]
pub struct BlockCache<R> {
    inner: Lock<RefCacheData<R>>,
}

/// cache of already loaded in-memory block `Ref`
struct RefCacheData<R> {
    entries: HashMap<HeaderHash, (R, delay_queue::Key)>,
    expirations: DelayQueue<HeaderHash>,

    ttl: Duration,
}

impl<R: Clone> BlockCache<R> {
    /// create a new `BlockCache` with the given expiration `Duration`.
    ///
    pub fn new(ttl: Duration) -> Self {
        BlockCache {
            inner: Lock::new(RefCacheData::new(ttl)),
        }
    }

    /// return a future that will attempt to insert the given value
    /// in the cache.
    ///
    /// # Errors
    ///
    /// there is no error possible yet.
    ///
    pub fn insert(&self, key: HeaderHash, value: R) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.insert(key, value))
    }

    /// Return a future to get a value from the cache.
    ///
    /// The future returns `None` if the entry was not found in the
    /// cache. This does not mean the associated block is not in the
    /// blockchain storage. It only means it is not in the cache:
    /// it has not been seen _recently_.
    ///
    /// # Errors
    ///
    /// No error possible yet
    ///
    pub fn get(&self, key: HeaderHash) -> impl Future<Item = Option<R>, Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.get(&key).cloned())
    }

    /// return a future to remove a specific entry from the cache.
    ///
    pub fn remove(&self, key: HeaderHash) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).map(move |mut guard| guard.remove(&key))
    }

    /// return a future that will remove every expired [`Ref`] from the cache
    ///
    pub fn purge(&self) -> impl Future<Item = (), Error = timer::Error> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .and_then(|mut guard| future::poll_fn(move || guard.poll_purge()))
    }
}

impl<R> RefCacheData<R> {
    fn new(ttl: Duration) -> Self {
        RefCacheData {
            entries: HashMap::new(),
            expirations: DelayQueue::new(),
            ttl,
        }
    }

    fn insert(&mut self, key: HeaderHash, value: R) {
        let delay = self.expirations.insert(key.clone(), self.ttl);

        self.entries.insert(key, (value, delay));
    }

    fn get(&mut self, key: &HeaderHash) -> Option<&R> {
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
