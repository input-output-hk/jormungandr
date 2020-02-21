use crate::{blockcfg::HeaderHash, blockchain::Ref};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::{prelude::*, sync::lock::Lock};

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
    inner: Lock<RefCacheData>,
}

/// cache of already loaded in-memory block `Ref`
struct RefCacheData {
    entries: HashMap<HeaderHash, Arc<Ref>>,
}

impl RefCache {
    /// create a new `RefCache`.
    ///
    pub fn new() -> Self {
        RefCache {
            inner: Lock::new(RefCacheData::new()),
        }
    }

    /// return a future that will attempt to insert the given [`Ref`]
    /// in the cache.
    ///
    /// # Errors
    ///
    /// there is no error possible yet.
    ///
    pub fn insert(
        &self,
        key: HeaderHash,
        value: Arc<Ref>,
    ) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();
        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.insert(key, value))
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
    pub fn get(&self, key: HeaderHash) -> impl Future<Item = Option<Arc<Ref>>, Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.get(&key).map(Arc::clone))
    }
}

impl RefCacheData {
    fn new() -> Self {
        RefCacheData {
            entries: HashMap::new(),
        }
    }

    fn insert(&mut self, key: HeaderHash, value: Arc<Ref>) {
        self.entries.insert(key, value);
    }

    fn get(&mut self, key: &HeaderHash) -> Option<&Arc<Ref>> {
        self.entries.get(key)
    }
}
