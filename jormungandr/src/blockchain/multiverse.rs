use crate::blockcfg::{ChainLength, HeaderHash, Multiverse as MultiverseData};
use chain_impl_mockchain::multiverse;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Multiverse<T> {
    inner: Arc<RwLock<MultiverseData<T>>>,
}

impl<T> Default for Multiverse<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Multiverse<T> {
    pub fn new() -> Self {
        Multiverse {
            inner: Arc::new(RwLock::new(MultiverseData::new())),
        }
    }

    pub async fn insert(
        &self,
        chain_length: ChainLength,
        hash: HeaderHash,
        value: T,
    ) -> multiverse::Ref<T> {
        let mut guard = self.inner.write().await;
        guard.insert(chain_length, hash, value)
    }

    pub async fn get_ref(&self, hash: HeaderHash) -> Option<multiverse::Ref<T>> {
        let guard = self.inner.read().await;
        guard.get_ref(&hash)
    }
}

impl<T: Clone> Multiverse<T> {
    pub async fn get(&self, hash: HeaderHash) -> Option<T> {
        let guard = self.inner.read().await;
        guard.get(&hash).as_deref().cloned()
    }
}

impl<T> Multiverse<T> {
    /// run the garbage collection of the multiverse
    ///
    pub async fn gc(&self, depth: u32) {
        let mut guard = self.inner.write().await;
        guard.gc(depth)
    }
}

impl<T> Clone for Multiverse<T> {
    fn clone(&self) -> Self {
        Multiverse {
            inner: self.inner.clone(),
        }
    }
}
