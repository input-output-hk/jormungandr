use crate::fragment::FragmentId;
use jormungandr_lib::interfaces::{FragmentLog, FragmentStatus};
use std::time::Duration;
use tokio::{
    prelude::*,
    sync::lock::{Lock, LockGuard},
    timer,
};

#[derive(Clone)]
pub struct Logs(Lock<internal::Logs>);

impl Logs {
    pub fn new(ttl: Duration) -> Self {
        Logs(Lock::new(internal::Logs::new(ttl)))
    }

    /// Returns true if fragment was registered
    pub fn insert(&mut self, log: FragmentLog) -> impl Future<Item = bool, Error = ()> {
        self.run_on_inner(move |inner| inner.insert(log))
    }

    /// Returns number of registered fragments
    pub fn insert_all(
        &mut self,
        logs: impl IntoIterator<Item = FragmentLog>,
    ) -> impl Future<Item = usize, Error = ()> {
        self.run_on_inner(move |inner| inner.insert_all(logs))
    }

    pub fn exists(&self, fragment_id: FragmentId) -> impl Future<Item = bool, Error = ()> {
        self.run_on_inner(move |inner| inner.exists(&fragment_id.into()))
    }

    pub fn exist_all(
        &self,
        fragment_ids: impl IntoIterator<Item = FragmentId>,
    ) -> impl Future<Item = Vec<bool>, Error = ()> {
        let hashes = fragment_ids.into_iter().map(Into::into);
        self.run_on_inner(move |inner| inner.exist_all(hashes))
    }

    pub fn modify(
        &mut self,
        fragment_id: FragmentId,
        status: FragmentStatus,
    ) -> impl Future<Item = (), Error = ()> {
        self.run_on_inner(move |inner| inner.modify(&fragment_id.into(), status))
    }

    pub fn modify_all(
        &mut self,
        fragment_ids: impl IntoIterator<Item = FragmentId>,
        status: FragmentStatus,
    ) -> impl Future<Item = (), Error = ()> {
        self.run_on_inner(move |inner| {
            for fragment_id in fragment_ids {
                let id = fragment_id.into();
                inner.modify(&id, status.clone())
            }
        })
    }

    pub fn remove(&mut self, fragment_id: FragmentId) -> impl Future<Item = (), Error = ()> {
        self.run_on_inner(move |inner| inner.remove(&fragment_id.into()))
    }

    pub fn poll_purge(&mut self) -> impl Future<Item = (), Error = timer::Error> {
        self.inner()
            .and_then(move |mut guard| future::poll_fn(move || guard.poll_purge()))
    }

    pub fn logs(&self) -> impl Future<Item = Vec<FragmentLog>, Error = ()> {
        self.run_on_inner(move |inner| inner.logs().cloned().collect())
    }

    fn run_on_inner<O>(
        &self,
        run: impl FnOnce(&mut internal::Logs) -> O,
    ) -> impl Future<Item = O, Error = ()> {
        self.inner()
            .and_then(move |mut guard| future::ok(run(&mut *guard)))
    }

    pub(super) fn inner<E>(&self) -> impl Future<Item = LockGuard<internal::Logs>, Error = E> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
    }
}

pub(super) mod internal {
    use jormungandr_lib::{
        crypto::hash::Hash,
        interfaces::{FragmentLog, FragmentOrigin, FragmentStatus},
    };
    use std::{
        collections::hash_map::{Entry, HashMap},
        time::{Duration, Instant},
    };
    use tokio::{
        prelude::*,
        timer::{self, delay_queue, DelayQueue},
    };

    pub struct Logs {
        entries: HashMap<Hash, (FragmentLog, delay_queue::Key)>,
        expirations: DelayQueue<Hash>,
        ttl: Duration,
    }

    impl Logs {
        pub fn new(ttl: Duration) -> Self {
            Logs {
                entries: HashMap::new(),
                expirations: DelayQueue::new(),
                ttl,
            }
        }

        pub fn exists(&self, fragment_id: &Hash) -> bool {
            self.entries.contains_key(fragment_id)
        }

        pub fn exist_all(&self, fragment_ids: impl IntoIterator<Item = Hash>) -> Vec<bool> {
            fragment_ids
                .into_iter()
                .map(|fragment_id| self.exists(&fragment_id))
                .collect()
        }

        /// Returns true if fragment was registered
        pub fn insert(&mut self, log: FragmentLog) -> bool {
            let fragment_id = *log.fragment_id();
            let entry = match self.entries.entry(fragment_id) {
                Entry::Occupied(_) => return false,
                Entry::Vacant(entry) => entry,
            };
            let delay = self.expirations.insert(fragment_id, self.ttl);
            entry.insert((log, delay));
            true
        }

        /// Returns number of registered fragments
        pub fn insert_all(&mut self, logs: impl IntoIterator<Item = FragmentLog>) -> usize {
            logs.into_iter()
                .map(|log| self.insert(log))
                .filter(|was_modified| *was_modified)
                .count()
        }

        pub fn modify(&mut self, fragment_id: &Hash, status: FragmentStatus) {
            match self.entries.entry(fragment_id.clone()) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().0.modify(status);

                    self.expirations
                        .reset_at(&entry.get().1, Instant::now() + self.ttl);
                }
                Entry::Vacant(entry) => {
                    // while a log modification, if the log was not already present in the
                    // logs it means we received it from the a new block from the network.
                    // we can mark the status of the transaction so newly received transaction
                    // be stored.

                    let delay = self.expirations.insert(*fragment_id, self.ttl);
                    entry.insert((
                        FragmentLog::new(fragment_id.clone().into_hash(), FragmentOrigin::Network),
                        delay,
                    ));
                }
            }
        }

        pub fn remove(&mut self, fragment_id: &Hash) {
            if let Some((_, cache_key)) = self.entries.remove(fragment_id) {
                self.expirations.remove(&cache_key);
            }
        }

        pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
            loop {
                match self.expirations.poll()? {
                    Async::NotReady => return Ok(Async::Ready(())),
                    Async::Ready(None) => return Ok(Async::Ready(())),
                    Async::Ready(Some(entry)) => {
                        self.entries.remove(entry.get_ref());
                    }
                }
            }
        }

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a FragmentLog> {
            self.entries.values().map(|(v, _)| v)
        }
    }
}
