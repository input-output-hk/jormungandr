use crate::fragment::{FragmentId, Log, Status};
use std::time::Duration;
use tokio::{prelude::*, sync::lock::Lock, timer};

#[derive(Clone)]
pub struct Logs(Lock<internal::Logs>);

impl Logs {
    pub fn new(ttl: Duration) -> Self {
        Logs(Lock::new(internal::Logs::new(ttl)))
    }

    pub fn insert(&mut self, log: Log) -> impl Future<Item = (), Error = ()> {
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
        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(move |guard| future::ok(guard.exists(fragment_ids)))
    }

    pub fn modify(
        &mut self,
        fragment_id: FragmentId,
        status: Status,
    ) -> impl Future<Item = (), Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |mut guard| {
            guard.modify(&fragment_id, status);
            future::ok(())
        })
    }

    pub fn remove(&mut self, fragment_id: FragmentId) -> impl Future<Item = (), Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock())).and_then(move |mut guard| {
            guard.remove(&fragment_id);
            future::ok(())
        })
    }

    pub fn poll_purge(&mut self) -> impl Future<Item = (), Error = timer::Error> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(move |mut guard| future::poll_fn(move || guard.poll_purge()))
    }

    pub fn logs(&self) -> impl Future<Item = Vec<Log>, Error = ()> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(|guard| future::ok(guard.logs().cloned().collect()))
    }
}

mod internal {
    use crate::fragment::{FragmentId, Log, Status};
    use std::{
        collections::HashMap,
        time::{Duration, Instant, SystemTime},
    };
    use tokio::{
        prelude::*,
        timer::{self, delay_queue, DelayQueue},
    };

    pub struct Logs {
        entries: HashMap<FragmentId, (Log, delay_queue::Key)>,
        expirations: DelayQueue<FragmentId>,
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

        pub fn exists(&self, fragment_ids: Vec<FragmentId>) -> Vec<bool> {
            fragment_ids
                .into_iter()
                .map(|id| self.entries.contains_key(&id))
                .collect()
        }

        pub fn insert(&mut self, log: Log) {
            let fragment_id = log.fragment_id.clone();
            let delay = self.expirations.insert(fragment_id.clone(), self.ttl);

            self.entries.insert(fragment_id, (log, delay));
        }

        pub fn modify(&mut self, fragment_id: &FragmentId, status: Status) {
            if let Some((ref mut log, ref key)) = self.entries.get_mut(fragment_id) {
                log.status = status;
                log.last_updated_at = SystemTime::now();

                self.expirations.reset_at(key, Instant::now() + self.ttl);
            } else {
                unimplemented!()
            }
        }

        pub fn remove(&mut self, fragment_id: &FragmentId) {
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

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a Log> {
            self.entries.values().map(|(v, _)| v)
        }
    }
}
