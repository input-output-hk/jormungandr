use crate::{
    blockcfg::{BlockDate, Ledger, LedgerParameters},
    fragment::{
        selection::{FragmentSelectionAlgorithm, FragmentSelectionAlgorithmParams, OldestFirst},
        Fragment, FragmentId, Logs,
    },
    intercom::{NetworkMsg, PropagateMsg},
    utils::async_msg::MessageBox,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{fragment::Contents, transaction::Transaction};
use futures03::{compat::*, sink::SinkExt};
use jormungandr_lib::interfaces::{FragmentLog, FragmentOrigin, FragmentStatus};
use slog::Logger;
use std::time::Duration;

pub struct Pool {
    logs: Logs,
    pool: internal::Pool,
    network_msg_box: MessageBox<NetworkMsg>,
}

impl Pool {
    pub fn new(
        max_entries: usize,
        ttl: Duration,
        gc_interval: Duration,
        logs: Logs,
        network_msg_box: MessageBox<NetworkMsg>,
    ) -> Self {
        Pool {
            logs,
            pool: internal::Pool::new(max_entries, ttl, gc_interval),
            network_msg_box,
        }
    }

    pub fn logs(&mut self) -> &mut Logs {
        &mut self.logs
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
        let mut network_msg_box = self.network_msg_box.clone().sink_compat();
        let fragment_ids = fragments.iter().map(Fragment::id).collect::<Vec<_>>();
        let fragments_exist_in_logs = self.logs.exist_all(fragment_ids);
        let new_fragments = fragments
            .into_iter()
            .zip(fragments_exist_in_logs)
            .filter(|(_, exists_in_logs)| !exists_in_logs)
            .map(|(fragment, _)| fragment);
        let new_fragments = self.pool.insert_all(new_fragments);
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
        self.logs.insert_all(fragment_logs);
        Ok(count)
    }

    pub fn remove_added_to_block(&mut self, fragment_ids: Vec<FragmentId>, status: FragmentStatus) {
        self.pool.remove_all(fragment_ids.iter().cloned());
        self.logs.modify_all(fragment_ids, status);
    }

    pub fn purge(&mut self) {
        self.pool.purge();
        self.logs.purge();
    }

    pub fn select(
        &mut self,
        ledger: Ledger,
        block_date: BlockDate,
        ledger_params: LedgerParameters,
        selection_alg: FragmentSelectionAlgorithmParams,
    ) -> Contents {
        let Pool { logs, pool, .. } = self;
        match selection_alg {
            FragmentSelectionAlgorithmParams::OldestFirst => {
                let mut selection_alg = OldestFirst::new();
                selection_alg.select(&ledger, &ledger_params, block_date, logs, pool);
                selection_alg.finalize()
            }
        }
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
    use crate::fragment::{
        expirations::{Expirations, Key},
        PoolEntry,
    };
    use std::{
        collections::{hash_map::Entry, HashMap, VecDeque},
        sync::Arc,
    };

    pub struct Pool {
        max_entries: usize,
        entries: HashMap<FragmentId, (Arc<PoolEntry>, Fragment, Key)>,
        entries_by_time: VecDeque<FragmentId>,
        expirations: Expirations<FragmentId>,
        ttl: Duration,
    }

    impl Pool {
        pub fn new(max_entries: usize, ttl: Duration, gc_interval: Duration) -> Self {
            Pool {
                max_entries,
                entries: HashMap::new(),
                entries_by_time: VecDeque::new(),
                expirations: Expirations::new(gc_interval),
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
                self.expirations.remove(cache_key);
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
            self.expirations.remove(cache_key);
            Some(fragment)
        }

        pub fn purge(&mut self) {
            for entry in self.expirations.pop_expired() {
                self.entries.remove(&entry);
                self.entries_by_time
                    .iter()
                    .position(|id| id == &entry)
                    .map(|position| {
                        self.entries_by_time.remove(position);
                    });
            }
        }
    }
}
