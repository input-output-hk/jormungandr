use jormungandr_lib::interfaces::{LeadershipLog, LeadershipLogId};
use std::time::Duration;
use tokio::{
    prelude::*,
    sync::lock::{Lock, LockGuard},
    timer,
};

/// all leadership logs, allow for following up on the different entity
/// of the blockchain
#[derive(Clone)]
pub struct Logs(Lock<internal::Logs>);

/// leadership log handle. will allow to update the status of the log
/// without having to hold the [`Logs`]
///
/// [`Logs`]: ./struct.Logs.html
pub struct LeadershipLogHandle {
    internal_id: LeadershipLogId,
    logs: Logs,
}

impl LeadershipLogHandle {
    /// make a leadership event as triggered.
    ///
    /// This should be called when the leadership event has started.
    ///
    /// # panic
    ///
    /// on non-release build, this function will panic if the log was already
    /// marked as awaken.
    ///
    pub fn mark_wake(&self) -> impl Future<Item = (), Error = ()> {
        self.logs.mark_wake(self.internal_id)
    }

    /// make a leadership event as finished.
    ///
    /// This should be called when the leadership event has finished its
    /// scheduled action.
    ///
    /// # panic
    ///
    /// on non-release build, this function will panic if the log was already
    /// marked as finished.
    ///
    pub fn mark_finished(&self) -> impl Future<Item = (), Error = ()> {
        self.logs.mark_finished(self.internal_id)
    }
}

impl Logs {
    /// create a Leadership Logs. This will make sure we delete from time to time
    /// some of the logs that are not necessary.
    ///
    /// the `ttl` can be any sensible value the user will see appropriate. The log will
    /// live at least its scheduled time + `ttl`.
    ///
    /// On changes, the log's TTL will be reset to this `ttl`.
    pub fn new(ttl: Duration) -> Self {
        Logs(Lock::new(internal::Logs::new(ttl)))
    }

    pub fn insert(
        &self,
        log: LeadershipLog,
    ) -> impl Future<Item = LeadershipLogHandle, Error = ()> {
        let logs = self.clone();
        self.inner().and_then(move |mut guard| {
            let id = guard.insert(log);

            future::ok(LeadershipLogHandle {
                internal_id: id,
                logs: logs,
            })
        })
    }

    fn mark_wake(&self, leadership_log_id: LeadershipLogId) -> impl Future<Item = (), Error = ()> {
        self.inner().and_then(move |mut guard| {
            guard.mark_wake(&leadership_log_id.into());
            future::ok(())
        })
    }

    fn mark_finished(
        &self,
        leadership_log_id: LeadershipLogId,
    ) -> impl Future<Item = (), Error = ()> {
        self.inner().and_then(move |mut guard| {
            guard.mark_finished(&leadership_log_id.into());
            future::ok(())
        })
    }

    pub fn poll_purge(&mut self) -> impl Future<Item = (), Error = timer::Error> {
        self.inner()
            .and_then(move |mut guard| future::poll_fn(move || guard.poll_purge()))
    }

    pub fn logs(&self) -> impl Future<Item = Vec<LeadershipLog>, Error = ()> {
        self.inner()
            .and_then(|guard| future::ok(guard.logs().cloned().collect()))
    }

    fn inner<E>(&self) -> impl Future<Item = LockGuard<internal::Logs>, Error = E> {
        let mut lock = self.0.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
    }
}

pub(super) mod internal {
    use super::{LeadershipLog, LeadershipLogId};
    use std::{
        collections::HashMap,
        time::{Duration, Instant},
    };
    use tokio::{
        prelude::*,
        timer::{self, delay_queue, DelayQueue},
    };

    pub struct Logs {
        entries: HashMap<LeadershipLogId, (LeadershipLog, delay_queue::Key)>,
        expirations: DelayQueue<LeadershipLogId>,
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

        pub fn insert(&mut self, log: LeadershipLog) -> LeadershipLogId {
            let id = log.leadership_log_id();

            let now = std::time::SystemTime::now();
            let minimal_duration = if &now < log.scheduled_at_time().as_ref() {
                log.scheduled_at_time()
                    .as_ref()
                    .duration_since(now)
                    .unwrap()
            } else {
                Duration::from_secs(0)
            };
            let ttl = minimal_duration.checked_add(self.ttl).unwrap_or(self.ttl);

            let delay = self.expirations.insert(id.clone(), ttl);

            self.entries.insert(id, (log, delay));
            id
        }

        pub fn mark_wake(&mut self, leadership_log_id: &LeadershipLogId) {
            if let Some((ref mut log, ref key)) = self.entries.get_mut(leadership_log_id) {
                log.mark_wake();

                self.expirations.reset_at(key, Instant::now() + self.ttl);
            } else {
                unimplemented!()
            }
        }

        pub fn mark_finished(&mut self, leadership_log_id: &LeadershipLogId) {
            if let Some((ref mut log, ref key)) = self.entries.get_mut(leadership_log_id) {
                log.mark_finished();

                self.expirations.reset_at(key, Instant::now() + self.ttl);
            } else {
                unimplemented!()
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

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a LeadershipLog> {
            self.entries.values().map(|(v, _)| v)
        }
    }
}
