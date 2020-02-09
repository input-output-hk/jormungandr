use crate::{
    blockcfg::{BlockDate, Ledger, LedgerParameters},
    fragment::{selection::FragmentSelectionAlgorithm, Fragment, FragmentId, Logs},
    intercom::{NetworkMsg, PropagateMsg},
    utils::async_msg::MessageBox,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::transaction::Transaction;
use futures03::{compat::*, future, sink::SinkExt};
use jormungandr_lib::interfaces::{FragmentLog, FragmentOrigin, FragmentStatus};
use slog::Logger;
use std::{sync::Arc, time::Duration};
use tokio02::{sync::Mutex, time};

#[derive(Clone)]
pub struct Pool {
    logs: Logs,
    pool: Arc<Mutex<internal::Pool>>,
    network_msg_box: MessageBox<NetworkMsg>,
}

impl Pool {
    pub fn new(
        max_entries: usize,
        ttl: Duration,
        logs: Logs,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        Pool {
            logs,
            pool: Arc::new(Mutex::new(internal::Pool::new(max_entries, ttl))),
            network_msg_box,
        }
    }

    pub fn logs(&self) -> &Logs {
        &self.logs
    }

    /// Returns number of registered fragments
    pub async fn insert_and_propagate_all(
        &mut self,
        origin: FragmentOrigin,
        mut fragments: Vec<Fragment>,
        logger: Logger,
    ) -> Result<usize, ()> {
        fragments.retain(is_fragment_valid);
        if fragments.is_empty() {
            return Ok(0);
        }
        let mut logs = self.logs.clone();
        let mut network_msg_box = self.network_msg_box.clone().sink_compat();
        let fragment_ids = fragments.iter().map(Fragment::id).collect::<Vec<_>>();
        let fragments_exist_in_logs = self.logs.exist_all(fragment_ids).await?;
        let mut pool = self.pool.lock().await;
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
        for fragment in new_fragments.into_iter() {
            let fragment_msg = NetworkMsg::Propagate(PropagateMsg::Fragment(fragment));
            network_msg_box
                .send(fragment_msg)
                .await
                .map_err(|e| error!(logger, "cannot propagate fragment to network: {}", e))?;
        }
        logs.insert_all(fragment_logs).await?;
        Ok(count)
    }

    pub async fn remove_added_to_block(
        &mut self,
        fragment_ids: Vec<FragmentId>,
        status: FragmentStatus,
    ) -> Result<(), ()> {
        let mut pool = self.pool.lock().await;
        pool.remove_all(fragment_ids.iter().cloned());
        self.logs.modify_all(fragment_ids, status).await
    }

    pub async fn poll_purge(&mut self) -> Result<(), time::Error> {
        let mut pool = self.pool.lock().await;
        future::poll_fn(move |cx| pool.poll_purge(cx)).await?;
        self.logs.poll_purge().await
    }

    pub async fn select<SelectAlg>(
        &mut self,
        ledger: Ledger,
        block_date: BlockDate,
        ledger_params: LedgerParameters,
        mut selection_alg: SelectAlg,
    ) -> Result<SelectAlg, ()>
    where
        SelectAlg: FragmentSelectionAlgorithm,
    {
        // FIXME deadlock hazard, nested pool lock and logs lock
        let mut pool = self.pool.lock().await;
        let mut logs = self.logs().inner().await;
        selection_alg.select(&ledger, &ledger_params, block_date, &mut logs, &mut pool);
        Ok(selection_alg)
    }
}

fn is_fragment_valid(fragment: &Fragment) -> bool {
    match fragment {
        // never valid in the pool, only acceptable in genesis
        Fragment::Initial(_) => false,
        Fragment::OldUtxoDeclaration(_) => false,
        // general transactions stuff
        Fragment::Transaction(ref tx) => is_transaction_valid(tx),
        Fragment::StakeDelegation(ref tx) => is_transaction_valid(tx),
        Fragment::OwnerStakeDelegation(ref tx) => is_transaction_valid(tx),
        Fragment::PoolRegistration(ref tx) => is_transaction_valid(tx),
        Fragment::PoolRetirement(ref tx) => is_transaction_valid(tx),
        // disabled for now
        Fragment::PoolUpdate(_) => false,
        Fragment::UpdateProposal(_) => false,
        Fragment::UpdateVote(_) => false,
    }
}

fn is_transaction_valid<E>(tx: &Transaction<E>) -> bool {
    tx.verify_possibly_balanced().is_ok()
}

pub(super) mod internal {
    use super::*;
    use crate::fragment::PoolEntry;
    use futures03::{
        stream::Stream,
        task::{Context, Poll},
    };
    use std::{
        collections::{hash_map::Entry, HashMap, VecDeque},
        pin::Pin,
        sync::Arc,
    };
    use tokio02::time::{delay_queue, DelayQueue};

    pub struct Pool {
        max_entries: usize,
        entries: HashMap<FragmentId, (Arc<PoolEntry>, Fragment, delay_queue::Key)>,
        entries_by_time: VecDeque<FragmentId>,
        expirations: Pin<Box<DelayQueue<FragmentId>>>,
        ttl: Duration,
    }

    impl Pool {
        pub fn new(max_entries: usize, ttl: Duration) -> Self {
            Pool {
                max_entries,
                entries: HashMap::new(),
                entries_by_time: VecDeque::new(),
                expirations: Box::pin(DelayQueue::new()),
                ttl,
            }
        }

        /// Returns clone of fragment if it was registered
        pub fn insert(&mut self, fragment: Fragment) -> Option<Fragment> {
            if self.max_entries < self.entries.len() {
                None
            } else {
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
        }

        /// Returns clones of registered fragments
        pub fn insert_all(
            &mut self,
            fragments: impl IntoIterator<Item = Fragment>,
        ) -> Vec<Fragment> {
            fragments
                .into_iter()
                .take(
                    self.max_entries
                        .checked_sub(self.entries.len())
                        .unwrap_or(0),
                )
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

        pub fn remove_all(&mut self, fragment_ids: impl IntoIterator<Item = FragmentId>) {
            // TODO fix terrible performance, entries_by_time are linear searched N times
            for fragment_id in fragment_ids {
                self.remove(&fragment_id);
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

        pub fn poll_purge(&mut self, cx: &mut Context) -> Poll<Result<(), time::Error>> {
            loop {
                match self.expirations.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(entry))) => {
                        self.entries.remove(entry.get_ref());
                        self.entries_by_time
                            .iter()
                            .position(|id| id == entry.get_ref())
                            .map(|position| {
                                self.entries_by_time.remove(position);
                            });
                    }
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                    Poll::Ready(None) => return Poll::Ready(Ok(())),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }
}
