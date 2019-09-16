use crate::{
    blockcfg::{HeaderContentEvalContext, Ledger, LedgerParameters},
    fragment::{selection::FragmentSelectionAlgorithm, Fragment, Logs},
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::transaction::{AuthenticatedTransaction, Balance};
use chain_impl_mockchain::value::Value;
use futures::future::{
    self,
    Either::{A, B},
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
        if !is_fragment_valid(&fragment) {
            return A(future::ok(false));
        }
        let mut pool_lock = self.pool.clone();
        let mut logs = self.logs.clone();
        B(self
            .logs
            .exists(fragment.id())
            .and_then(move |exists_in_logs| {
                if exists_in_logs {
                    return A(future::ok(false));
                }
                B(
                    future::poll_fn(move || Ok(pool_lock.poll_lock())).and_then(move |mut pool| {
                        let fragment = match pool.insert(fragment) {
                            Some(fragment) => fragment,
                            None => return A(future::ok(false)),
                        };
                        let fragment_log = FragmentLog::new(fragment.id().into(), origin);
                        let insert_future = logs.insert(fragment_log).map(|_| true);
                        B(insert_future)
                    }),
                )
            }))
    }

    /// Returns number of registered fragments
    pub fn insert_all(
        &mut self,
        origin: FragmentOrigin,
        mut fragments: Vec<Fragment>,
    ) -> impl Future<Item = usize, Error = ()> {
        fragments.retain(is_fragment_valid);
        if fragments.is_empty() {
            return A(future::ok(0));
        }
        let mut pool_lock = self.pool.clone();
        let mut logs = self.logs.clone();
        let fragment_ids = fragments.iter().map(Fragment::id).collect::<Vec<_>>();
        let fragments_exist_in_logs = self.logs.exist_all(fragment_ids);
        B(
            fragments_exist_in_logs.and_then(move |fragments_exist_in_logs| {
                future::poll_fn(move || Ok(pool_lock.poll_lock())).and_then(move |mut pool| {
                    let new_fragments = fragments
                        .into_iter()
                        .zip(fragments_exist_in_logs)
                        .filter(|(_, exists_in_logs)| !exists_in_logs)
                        .map(|(fragment, _)| fragment);
                    let new_fragments = pool.insert_all(new_fragments);
                    let count = new_fragments.len();
                    let fragment_logs = new_fragments
                        .iter()
                        .map(move |fragment| FragmentLog::new(fragment.id().into(), origin))
                        .collect::<Vec<_>>();
                    logs.insert_all(fragment_logs).map(move |_| count)
                })
            }),
        )
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

fn is_fragment_valid(fragment: &Fragment) -> bool {
    match fragment {
        Fragment::Transaction(ref tx) => is_transaction_valid(tx),
        _ => true,
    }
}

fn is_transaction_valid<A, E>(tx: &AuthenticatedTransaction<A, E>) -> bool {
    tx.transaction.verify_possibly_balanced().is_ok()
}

pub(super) mod internal {
    use super::*;
    use crate::fragment::{FragmentId, PoolEntry};
    use std::{
        collections::{hash_map::Entry, HashMap, VecDeque},
        sync::Arc,
    };
    use tokio::timer::{delay_queue, DelayQueue};

    pub struct Pool {
        entries: HashMap<FragmentId, (Arc<PoolEntry>, Fragment, delay_queue::Key)>,
        entries_by_time: VecDeque<FragmentId>,
        expirations: DelayQueue<FragmentId>,
        ttl: Duration,
    }

    impl Pool {
        pub fn new(ttl: Duration) -> Self {
            Pool {
                entries: HashMap::new(),
                entries_by_time: VecDeque::new(),
                expirations: DelayQueue::new(),
                ttl,
            }
        }

        /// Returns clone of fragment if it was registered
        pub fn insert(&mut self, fragment: Fragment) -> Option<Fragment> {
            let fragment_id = fragment.id();
            let entry = match self.entries.entry(fragment_id) {
                Entry::Occupied(_) => return None,
                Entry::Vacant(vacant) => vacant,
            };
            let pool_entry = Arc::new(PoolEntry::new(&fragment));
            let delay = self.expirations.insert(fragment_id, self.ttl);
            entry.insert((pool_entry, fragment.clone(), delay));
            self.entries_by_time.push_back(fragment_id);
            Some(fragment)
        }

        /// Returns clones of registered fragments
        pub fn insert_all(
            &mut self,
            fragments: impl IntoIterator<Item = Fragment>,
        ) -> Vec<Fragment> {
            fragments
                .into_iter()
                .filter_map(|fragment| self.insert(fragment))
                .collect()
        }

        pub fn remove(&mut self, fragment_id: &FragmentId) -> Option<Fragment> {
            if let Some((_, fragment, cache_key)) = self.entries.remove(fragment_id) {
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

        pub fn remove_oldest(&mut self) -> Option<Fragment> {
            let fragment_id = self.entries_by_time.pop_front()?;
            let (_, fragment, cache_key) = self
                .entries
                .remove(&fragment_id)
                .expect("Pool lost fragment ID consistency");
            self.expirations.remove(&cache_key);
            Some(fragment)
        }

        pub fn poll_purge(&mut self) -> Poll<(), timer::Error> {
            loop {
                match self.expirations.poll()? {
                    Async::NotReady => return Ok(Async::Ready(())),
                    Async::Ready(None) => return Ok(Async::Ready(())),
                    Async::Ready(Some(entry)) => {
                        self.entries.remove(entry.get_ref());
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
