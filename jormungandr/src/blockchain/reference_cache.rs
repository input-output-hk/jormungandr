use crate::{blockcfg::HeaderHash, blockchain::Ref};
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::Mutex;

/// object that store the [`Ref`] in a cache. Every time a [`Ref`]
/// is accessed its TTL will be reset. Once the TTL of [`Ref`] has
/// expired it may be removed from the cache.
///
/// The cache expired [`Ref`] will be removed only if the [`Ref`]'s
/// TTL has expired and [`purge`] has been called and has completed.
///
/// [`Ref`]: ./struct.Ref.html
/// [`purge`]: ./struct.Ref.html#method.purge
#[derive(Clone)]
pub struct RefCache {
    inner: Arc<Mutex<LruCache<HeaderHash, Arc<Ref>>>>,
}

impl RefCache {
    /// create a new `RefCache`.
    ///
    pub fn new(cap: usize) -> Self {
        RefCache {
            inner: Arc::new(Mutex::new(LruCache::new(cap))),
        }
    }

    /// return a future that will attempt to insert the given [`Ref`]
    /// in the cache.
    ///
    /// # Errors
    ///
    /// there is no error possible yet.
    ///
    pub async fn insert(&self, key: HeaderHash, value: Arc<Ref>) {
        let mut guard = self.inner.lock().await;
        guard.put(key, value);
    }

    /// return a future to get a [`Ref`] from the cache
    ///
    /// The future returns `None` if the `Ref` was not found in the
    /// cache. This does not mean the associated block is not in the
    /// blockchain storage. It only means it is not in the cache:
    /// it has not been seen _recently_.
    ///
    /// # Errors
    ///
    /// No error possible yet
    ///
    pub async fn get(&self, key: HeaderHash) -> Option<Arc<Ref>> {
        let mut guard = self.inner.lock().await;
        guard.get(&key).map(Arc::clone)
    }
}
