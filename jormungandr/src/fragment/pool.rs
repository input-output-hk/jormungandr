use crate::{
    blockcfg::{HeaderContentEvalContext, Ledger, LedgerParameters},
    fragment::{selection::FragmentSelectionAlgorithm, Fragment, Logs},
};
use jormungandr_lib::interfaces::{FragmentLog, FragmentOrigin};
use std::time::Duration;
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

    /// Returns true if fragment was registered
    pub fn insert(
        &mut self,
        origin: FragmentOrigin,
        fragment: Fragment,
    ) -> impl Future<Item = bool, Error = ()> {
        use chain_core::property::Fragment as _;

        let mut pool_lock = self.pool.clone();
        let mut logs = self.logs.clone();
        future::poll_fn(move || Ok(pool_lock.poll_lock())).and_then(move |mut pool| {
            let id = fragment.id();
            if pool.insert(fragment) == false {
                return future::Either::A(future::ok(false));
            }
            let fragment_log = FragmentLog::new(id.into(), origin);
            let insert_future = logs.insert(fragment_log).map(|_| true);
            future::Either::B(insert_future)
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
    use chain_core::property::Fragment as _;
    use std::{
        collections::{hash_map::Entry, BTreeMap, HashMap, VecDeque},
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

        pub fn insert(&mut self, fragment: Fragment) -> bool {
            let fragment_id = fragment.id();
            let entry = match self.entries.entry(fragment_id) {
                Entry::Occupied(_) => return false,
                Entry::Vacant(vacant) => vacant,
            };
            let pool_entry = Arc::new(PoolEntry::new(&fragment));
            let delay = self.expirations.insert(fragment_id, self.ttl);
            entry.insert((pool_entry.clone(), fragment, delay));
            self.entries_by_id.insert(fragment_id, pool_entry);
            self.entries_by_time.push_back(fragment_id);
            true
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
            loop {
                match self.expirations.poll()? {
                    Async::NotReady => return Ok(Async::Ready(())),
                    Async::Ready(None) => return Ok(Async::Ready(())),
                    Async::Ready(Some(entry)) => {
                        self.entries.remove(entry.get_ref());
                        self.entries_by_id.remove(entry.get_ref());
                        self.entries_by_time
                            .iter()
                            .position(|id| id == entry.get_ref())
                            .map(|position| {
                                self.entries_by_time.remove(position);
                            });
                    }
                }
            }
        }
    }
}
