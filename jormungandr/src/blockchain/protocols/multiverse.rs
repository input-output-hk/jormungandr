use crate::blockcfg::{ChainLength, HeaderHash, Ledger, Multiverse as MultiverseData};
use chain_impl_mockchain::multiverse::GCRoot;
use std::convert::Infallible;
use tokio::{prelude::*, sync::lock::Lock};

pub struct Multiverse<T> {
    inner: Lock<MultiverseData<T>>,
}

impl<T> Multiverse<T> {
    pub fn new() -> Self {
        Multiverse {
            inner: Lock::new(MultiverseData::new()),
        }
    }

    pub fn insert(
        &self,
        chain_length: ChainLength,
        hash: HeaderHash,
        value: T,
    ) -> impl Future<Item = GCRoot, Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock()))
            .map(move |mut guard| guard.insert(chain_length, hash, value))
    }
}

impl<T: Clone> Multiverse<T> {
    pub fn get(&self, hash: HeaderHash) -> impl Future<Item = Option<T>, Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).map(move |guard| guard.get(&hash).cloned())
    }
}

impl Multiverse<Ledger> {
    /// run the garbage collection of the multiverse
    ///
    /// TODO: this function is only working for the `Ledger` at the moment
    ///       we need to generalize the `chain_impl_mockchain` to handle
    ///       the garbage collection for any `T`
    pub fn purge(&self) -> impl Future<Item = (), Error = Infallible> {
        let mut inner = self.inner.clone();

        future::poll_fn(move || Ok(inner.poll_lock())).map(|mut guard| guard.gc())
    }
}

impl<T> Clone for Multiverse<T> {
    fn clone(&self) -> Self {
        Multiverse {
            inner: self.inner.clone(),
        }
    }
}
