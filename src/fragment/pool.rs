use crate::fragment::{Fragment, Log, Logs, Origin, Status};
use std::time::{Duration, SystemTime};
use tokio::{prelude::*, sync::lock::Lock, timer};

#[derive(Clone)]
pub struct Pool {
    logs: Logs,
    pool: Lock<internal::Pool>,
}

impl Pool {
    pub fn new(ttl: Duration, logs: Logs) -> Self {
        Pool {
            logs,
            pool: Lock::new(internal::Pool::new(ttl)),
        }
    }

    pub fn logs(&self) -> &Logs {
        &self.logs
    }

    pub fn insert(
        &mut self,
        origin: Origin,
        fragment: Fragment,
    ) -> impl Future<Item = bool, Error = ()> {
        use chain_core::property::Message as _;

        let id = fragment.id();
        let mut lock = self.pool.clone();
        let mut logs = self.logs.clone();

        self.logs()
            .exists(vec![id.clone()])
            .and_then(move |exists| {
                if exists[0] {
                    future::Either::A(future::ok(false))
                } else {
                    future::Either::B(future::poll_fn(move || Ok(lock.poll_lock())).and_then(
                        move |mut guard| {
                            guard.insert(fragment);

                            let log = Log {
                                fragment_id: id,
                                last_updated_at: SystemTime::now(),
                                received_at: SystemTime::now(),
                                received_from: origin,
                                status: Status::Pending,
                            };
                            logs.insert(log).map(|()| true)
                        },
                    ))
                }
            })
    }

    pub fn poll_purge(&mut self) -> impl Future<Item = (), Error = timer::Error> {
        let mut lock = self.pool.clone();
        let purge_logs = self.logs.poll_purge();

        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(move |mut guard| future::poll_fn(move || guard.poll_purge()))
            .and_then(move |()| purge_logs)
    }
}

mod internal {
    use crate::fragment::{Fragment, FragmentId, PoolEntry};
    use std::{collections::HashMap, time::Duration};
    use tokio::{
        prelude::*,
        timer::{self, delay_queue, DelayQueue},
    };

    pub struct Pool {
        entries: HashMap<FragmentId, (PoolEntry, Fragment, delay_queue::Key)>,
        expirations: DelayQueue<FragmentId>,
        ttl: Duration,
    }

    impl Pool {
        pub fn new(ttl: Duration) -> Self {
            Pool {
                entries: HashMap::new(),
                expirations: DelayQueue::new(),
                ttl,
            }
        }

        pub fn insert(&mut self, fragment: Fragment) {
            let entry = PoolEntry::new(&fragment);
            let fragment_id = entry.fragment_ref().clone();
            let delay = self.expirations.insert(fragment_id.clone(), self.ttl);

            self.entries.insert(fragment_id, (entry, fragment, delay));
        }

        pub fn remove(&mut self, fragment_id: &FragmentId) {
            if let Some((_, _, cache_key)) = self.entries.remove(fragment_id) {
                self.expirations.remove(&cache_key);
            }
        }

        pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
            while let Some(entry) = try_ready!(self.expirations.poll()) {
                self.entries.remove(entry.get_ref());
            }

            Ok(Async::Ready(()))
        }
    }
}
