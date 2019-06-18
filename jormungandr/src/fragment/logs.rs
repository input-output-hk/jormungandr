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

    pub fn insert(&mut self, log: FragmentLog) -> impl Future<Item = (), Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |mut guard| {
            guard.insert(log);
            future::ok(())
        })
    }

    pub fn exists(
        &self,
        fragment_ids: Vec<FragmentId>,
    ) -> impl Future<Item = Vec<bool>, Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |guard| {
            future::ok(guard.exists(fragment_ids.into_iter().map(|fids| fids.into())))
        })
    }

    pub fn modify(
        &mut self,
        fragment_id: FragmentId,
        status: FragmentStatus,
    ) -> impl Future<Item = (), Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |mut guard| {
            guard.modify(&fragment_id.into(), status);
            future::ok(())
        })
    }

    pub fn remove(&mut self, fragment_id: FragmentId) -> impl Future<Item = (), Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |mut guard| {
            guard.remove(&fragment_id.into());
            future::ok(())
        })
    }

    pub fn poll_purge(&mut self) -> impl Future<Item = (), Error = timer::Error> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(move |mut guard| future::poll_fn(move || guard.poll_purge()))
    }

    pub fn logs(&self) -> impl Future<Item = Vec<FragmentLog>, Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(|guard| future::ok(guard.logs().cloned().collect()))
    }

    pub(super) fn inner(&self) -> impl Future<Item = LockGuard<internal::Logs>, Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
    }
}

pub(super) mod internal {
    use jormungandr_lib::{
        crypto::hash::Hash,
        interfaces::{FragmentLog, FragmentStatus},
    };
    use std::{
        collections::HashMap,
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

        pub fn exists<I>(&self, fragment_ids: I) -> Vec<bool>
        where
            I: IntoIterator<Item = Hash>,
        {
            fragment_ids
                .into_iter()
                .map(|id| self.entries.contains_key(&id))
                .collect()
        }

        pub fn insert(&mut self, log: FragmentLog) {
            let fragment_id = log.fragment_id().clone();
            let delay = self.expirations.insert(fragment_id.clone(), self.ttl);

            self.entries.insert(fragment_id, (log, delay));
        }

        pub fn modify(&mut self, fragment_id: &Hash, status: FragmentStatus) {
            if let Some((ref mut log, ref key)) = self.entries.get_mut(fragment_id) {
                log.modify(status);

                self.expirations.reset_at(key, Instant::now() + self.ttl);
            } else {
                unimplemented!()
            }
        }

        pub fn remove(&mut self, fragment_id: &Hash) {
            if let Some((_, cache_key)) = self.entries.remove(fragment_id) {
                self.expirations.remove(&cache_key);
            }
        }

        pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
            while let Some(entry) = try_ready!(self.expirations.poll()) {
                self.entries.remove(entry.get_ref());
            }

            Ok(Async::Ready(()))
        }

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a FragmentLog> {
            self.entries.values().map(|(v, _)| v)
        }
    }
}
