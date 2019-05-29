use crate::{
    blockcfg::{HeaderContentEvalContext, Ledger, LedgerParameters},
    fragment::{selection::FragmentSelectionAlgorithm, Fragment, Log, Logs, Origin, Status},
};
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

    pub fn select<SelectAlg>(
        &mut self,
        ledger: Ledger,
        metadata: HeaderContentEvalContext,
        ledger_params: LedgerParameters,
        mut selection_alg: SelectAlg,
    ) -> impl Future<Item = SelectAlg, Error = ()>
    where
        SelectAlg: FragmentSelectionAlgorithm,
    {
        let mut lock = self.pool.clone();
        let logs = self.logs().clone();

        future::poll_fn(move || Ok(lock.poll_lock()))
            .and_then(move |pool| logs.inner().map(|logs| (pool, logs)))
            .and_then(move |(mut pool, mut logs)| {
                selection_alg.select(&ledger, &ledger_params, &metadata, &mut logs, &mut pool);
                future::ok(selection_alg)
            })
    }
}

pub(super) mod internal {
    use crate::fragment::{Fragment, FragmentId, PoolEntry};
    use std::{
        collections::{BTreeMap, HashMap, VecDeque},
        sync::Arc,
        time::Duration,
    };
    use tokio::{
        prelude::*,
        timer::{self, delay_queue, DelayQueue},
    };

    pub struct Pool {
        pub entries: HashMap<FragmentId, (Arc<PoolEntry>, Fragment, delay_queue::Key)>,
        pub entries_by_id: BTreeMap<FragmentId, Arc<PoolEntry>>,
        pub entries_by_time: VecDeque<FragmentId>,
        expirations: DelayQueue<FragmentId>,
        ttl: Duration,
    }

    impl Pool {
        pub fn new(ttl: Duration) -> Self {
            Pool {
                entries: HashMap::new(),
                entries_by_id: BTreeMap::new(),
                entries_by_time: VecDeque::new(),
                expirations: DelayQueue::new(),
                ttl,
            }
        }

        pub fn insert(&mut self, fragment: Fragment) {
            let entry = Arc::new(PoolEntry::new(&fragment));
            let fragment_id = entry.fragment_ref().clone();
            let delay = self.expirations.insert(fragment_id.clone(), self.ttl);

            self.entries
                .insert(fragment_id.clone(), (entry.clone(), fragment, delay));
            self.entries_by_id
                .insert(fragment_id.clone(), entry.clone());
            self.entries_by_time.push_back(fragment_id);
        }

        pub fn remove(&mut self, fragment_id: &FragmentId) -> Option<Fragment> {
            if let Some((_, fragment, cache_key)) = self.entries.remove(fragment_id) {
                self.entries_by_id.remove(fragment_id);
                self.entries_by_time
                    .iter()
                    .position(|id| id == fragment_id)
                    .map(|position| {
                        self.entries_by_time.remove(position);
                    });
                self.expirations.remove(&cache_key);
                Some(fragment)
            } else {
                None
            }
        }

        pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
            while let Some(entry) = try_ready!(self.expirations.poll()) {
                self.entries.remove(entry.get_ref());
                self.entries_by_id.remove(entry.get_ref());
                self.entries_by_time
                    .iter()
                    .position(|id| id == entry.get_ref())
                    .map(|position| {
                        self.entries_by_time.remove(position);
                    });
            }

            Ok(Async::Ready(()))
        }
    }
}
