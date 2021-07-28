use crate::{
    blockcfg::{ApplyBlockLedger, LedgerParameters},
    fragment::{
        selection::{FragmentSelectionAlgorithm, FragmentSelectionAlgorithmParams, OldestFirst},
        Fragment, FragmentId, Logs,
    },
    intercom::{NetworkMsg, PropagateMsg},
    utils::async_msg::MessageBox,
};
use chain_core::property::Fragment as _;
use chain_impl_mockchain::{fragment::Contents, transaction::Transaction};
use futures::channel::mpsc::SendError;
use jormungandr_lib::{
    interfaces::{
        BlockDate, FragmentLog, FragmentOrigin, FragmentRejectionReason, FragmentStatus,
        FragmentsProcessingSummary, PersistentFragmentLog, RejectedFragmentInfo,
    },
    time::SecondsSinceUnixEpoch,
};
use std::collections::HashMap;
use thiserror::Error;

use std::fs::File;
use std::mem;

pub struct Pools {
    logs: Logs,
    pools: Vec<internal::Pool>,
    network_msg_box: MessageBox<NetworkMsg>,
    persistent_log: Option<File>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot propagate a fragment to the network")]
    CannotPropagate(#[source] SendError),
}

impl Pools {
    pub fn new(
        max_entries: usize,
        n_pools: usize,
        logs: Logs,
        network_msg_box: MessageBox<NetworkMsg>,
        persistent_log: Option<File>,
    ) -> Self {
        // we need a pool even for passive nodes to be able to participate in
        // the fragments dissemination protocol
        let n_pools = std::cmp::max(1, n_pools);
        let pools = (0..n_pools)
            .map(|_| internal::Pool::new(max_entries))
            .collect();
        Pools {
            logs,
            pools,
            network_msg_box,
            persistent_log,
        }
    }

    pub fn logs(&mut self) -> &mut Logs {
        &mut self.logs
    }

    /// Sets the persistent log to a file.
    /// The file must be opened for writing.
    pub fn set_persistent_log(&mut self, file: File) {
        self.persistent_log = Some(file);
    }

    /// Synchronizes the persistent log file contents and metadata
    /// to the file system and closes the file.
    pub fn close_persistent_log(&mut self) {
        if let Some(file) = mem::replace(&mut self.persistent_log, None) {
            if let Err(e) = file.sync_all() {
                tracing::error!(error = %e, "failed to sync persistent log file");
            }
        }
    }

    fn filter_fragments(
        &mut self,
        origin: FragmentOrigin,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> (Vec<Fragment>, FragmentsProcessingSummary) {
        use bincode::Options;

        let mut filtered_fragments = Vec::new();
        let mut rejected = Vec::new();

        let mut fragments = fragments.into_iter();

        for fragment in fragments.by_ref() {
            let id = fragment.id();

            let span = tracing::trace_span!("pool_incoming_fragment", fragment_id=?id);
            let _enter = span.enter();

            if self.logs.exists(id) {
                rejected.push(RejectedFragmentInfo {
                    id,
                    reason: FragmentRejectionReason::FragmentAlreadyInLog,
                });
                tracing::debug!("fragment is already logged");
                continue;
            }

            if !is_fragment_valid(&fragment) {
                rejected.push(RejectedFragmentInfo {
                    id,
                    reason: FragmentRejectionReason::FragmentInvalid,
                });

                tracing::debug!("fragment is invalid, not including to the pool");

                if fail_fast {
                    tracing::debug!("fail_fast is enabled; rejecting all downstream fragments");
                    break;
                }

                continue;
            }

            if let Some(mut persistent_log) = self.persistent_log.as_mut() {
                let entry = PersistentFragmentLog {
                    time: SecondsSinceUnixEpoch::now(),
                    fragment: fragment.clone(),
                };
                // this must be sufficient: the PersistentFragmentLog format is using byte array
                // for serialization so we do not expect any problems during deserialization
                let codec = bincode::DefaultOptions::new().with_fixint_encoding();
                if let Err(err) = codec.serialize_into(&mut persistent_log, &entry) {
                    tracing::error!(err = %err, "failed to write persistent fragment log entry");
                }
            }

            tracing::debug!("including fragment to the pool");

            filtered_fragments.push(fragment);
        }

        if fail_fast {
            for fragment in fragments {
                let id = fragment.id();
                let span = tracing::trace_span!("pool_incoming_fragment", fragment_id=?id);
                let _enter = span.enter();
                tracing::error!(
                    "rejected due to fail_fast and one of previous fragments being invalid"
                );
                rejected.push(RejectedFragmentInfo {
                    id,
                    reason: FragmentRejectionReason::PreviousFragmentInvalid,
                })
            }
        }

        let mut accepted_fragments = HashMap::new();

        for (pool_number, pool) in self.pools.iter_mut().enumerate() {
            let span = tracing::trace_span!("pool_insert_fragment", pool_number=?pool_number);
            let _enter = span.enter();

            let mut fragments = filtered_fragments.clone().into_iter();
            let new_fragments = pool.insert_all(fragments.by_ref());
            let count = new_fragments.len();
            tracing::debug!("{} of the received fragments were added to the pool", count,);
            let fragment_logs: Vec<_> = new_fragments
                .iter()
                .map(move |fragment| FragmentLog::new(fragment.id(), origin))
                .collect();
            self.logs.insert_all_pending(fragment_logs);

            for fragment in new_fragments {
                let id = fragment.id();
                tracing::debug!(fragment_id=?id, "inserted fragment to the pool");
                accepted_fragments.insert(id, fragment);
            }

            for fragment in fragments {
                let id = fragment.id();
                tracing::debug!(fragment_id=?id, "rejecting fragment due to pool overflow");
                rejected.push(RejectedFragmentInfo {
                    id,
                    reason: FragmentRejectionReason::PoolOverflow { pool_number },
                })
            }
        }

        let (accepted, fragments) = accepted_fragments.into_iter().unzip();

        (fragments, FragmentsProcessingSummary { accepted, rejected })
    }

    /// Returns number of registered fragments. Setting `fail_fast` to `true` will force this
    /// method to reject all fragments after the first invalid fragments was met.
    pub async fn insert_and_propagate_all(
        &mut self,
        origin: FragmentOrigin,
        fragments: Vec<Fragment>,
        fail_fast: bool,
    ) -> Result<FragmentsProcessingSummary, Error> {
        tracing::debug!(origin = ?origin, "received {} fragments", fragments.len());

        let (filtered_fragments, summary) = self.filter_fragments(origin, fragments, fail_fast);

        let mut network_msg_box = self.network_msg_box.clone();

        for fragment in filtered_fragments.into_iter() {
            let fragment_msg = NetworkMsg::Propagate(PropagateMsg::Fragment(fragment));
            network_msg_box
                // do not block the current process if network queue is full
                .try_send(fragment_msg)
                .map_err(|e| Error::CannotPropagate(e.into_send_error()))?;
        }

        Ok(summary)
    }

    pub fn remove_added_to_block(&mut self, fragment_ids: Vec<FragmentId>, status: FragmentStatus) {
        let date = if let FragmentStatus::InABlock { date, .. } = status {
            date
        } else {
            panic!("expected status to be in block, found {:?}", status);
        };

        for pool in &mut self.pools {
            pool.remove_all(fragment_ids.iter());
        }

        self.logs.modify_all(fragment_ids, status, date);
    }

    pub async fn select(
        &mut self,
        pool_idx: usize,
        ledger: ApplyBlockLedger,
        ledger_params: LedgerParameters,
        selection_alg: FragmentSelectionAlgorithmParams,
        soft_deadline_future: futures::channel::oneshot::Receiver<()>,
        hard_deadline_future: futures::channel::oneshot::Receiver<()>,
    ) -> (Contents, ApplyBlockLedger) {
        let Pools { logs, pools, .. } = self;
        let pool = &mut pools[pool_idx];
        match selection_alg {
            FragmentSelectionAlgorithmParams::OldestFirst => {
                let mut selection_alg = OldestFirst::new();
                selection_alg
                    .select(
                        ledger,
                        &ledger_params,
                        logs,
                        pool,
                        soft_deadline_future,
                        hard_deadline_future,
                    )
                    .await
            }
        }
    }

    // Remove from logs fragments that were confirmed (or rejected) in a branch
    pub fn prune_after_ledger_branch(&mut self, branch_date: BlockDate) {
        self.logs.remove_logs_after_date(branch_date)
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
        Fragment::PoolUpdate(ref tx) => is_transaction_valid(tx),
        // vote stuff
        Fragment::UpdateProposal(_) => false, // TODO: enable when ready
        Fragment::UpdateVote(_) => false,     // TODO: enable when ready
        Fragment::VotePlan(ref tx) => is_transaction_valid(tx),
        Fragment::VoteCast(ref tx) => is_transaction_valid(tx),
        Fragment::VoteTally(ref tx) => is_transaction_valid(tx),
        Fragment::EncryptedVoteTally(ref tx) => is_transaction_valid(tx),
    }
}

fn is_transaction_valid<E>(tx: &Transaction<E>) -> bool {
    tx.verify_possibly_balanced().is_ok()
}

pub(super) mod internal {
    use super::*;

    use std::{
        collections::HashMap,
        hash::{Hash, Hasher},
        ptr,
    };

    /// Doubly-linked queue with the possibility to remove elements from the middle of the list by
    /// their keys.
    struct IndexedDeqeue<K, V> {
        head: *mut IndexedDequeueEntry<K, V>,
        tail: *mut IndexedDequeueEntry<K, V>,

        index: HashMap<IndexedDequeueKeyRef<K>, Box<IndexedDequeueEntry<K, V>>>,
    }

    struct IndexedDequeueEntry<K, V> {
        key: K,
        value: V,

        prev: *mut IndexedDequeueEntry<K, V>,
        next: *mut IndexedDequeueEntry<K, V>,
    }

    /// A wrapper around the pointer to the key of the queue element. This wrapper forwards the
    /// implementations of `Eq` and `Hash` to `K`. This is required becuase by default the
    /// implementations of `Eq` and `Hash` from the pointer itself will be used.
    struct IndexedDequeueKeyRef<K>(*const K);

    impl<K, V> IndexedDeqeue<K, V>
    where
        K: Eq + Hash,
    {
        fn new() -> Self {
            Self {
                head: ptr::null_mut(),
                tail: ptr::null_mut(),

                index: HashMap::new(),
            }
        }

        fn push_front(&mut self, key: K, value: V) {
            let mut entry = Box::new(IndexedDequeueEntry {
                key,
                value,
                prev: ptr::null_mut(),
                next: self.head,
            });
            if let Some(head) = unsafe { self.head.as_mut() } {
                head.prev = &mut *entry;
            } else {
                self.tail = &mut *entry;
            }
            self.head = &mut *entry;
            if self
                .index
                .insert(IndexedDequeueKeyRef(&entry.key), entry)
                .is_some()
            {
                panic!("inserted an already existing key");
            }
        }

        fn push_back(&mut self, key: K, value: V) {
            let mut entry = Box::new(IndexedDequeueEntry {
                key,
                value,
                prev: self.tail,
                next: ptr::null_mut(),
            });
            if let Some(tail) = unsafe { self.tail.as_mut() } {
                tail.next = &mut *entry;
            } else {
                self.head = &mut *entry;
            }
            self.tail = &mut *entry;
            if self
                .index
                .insert(IndexedDequeueKeyRef(&entry.key), entry)
                .is_some()
            {
                panic!("inserted an already existing key");
            }
        }

        fn pop_back(&mut self) -> Option<(K, V)> {
            let tail = unsafe { self.tail.as_mut() }?;
            self.tail = tail.prev;
            if let Some(prev) = unsafe { tail.prev.as_mut() } {
                prev.next = ptr::null_mut();
            } else {
                self.head = ptr::null_mut();
            }
            let entry = self.index.remove(&IndexedDequeueKeyRef(&tail.key)).unwrap();
            Some((entry.key, entry.value))
        }

        fn remove(&mut self, key: &K) -> Option<V> {
            let entry = self.index.remove(&IndexedDequeueKeyRef(key))?;
            if let Some(prev) = unsafe { entry.prev.as_mut() } {
                prev.next = entry.next;
            } else {
                self.head = entry.next;
            }
            if let Some(next) = unsafe { entry.next.as_mut() } {
                next.prev = entry.prev;
            } else {
                self.tail = entry.prev;
            }
            Some(entry.value)
        }

        fn len(&self) -> usize {
            self.index.len()
        }

        fn contains(&self, key: &K) -> bool {
            self.index.contains_key(&IndexedDequeueKeyRef(key))
        }
    }

    unsafe impl<K: Send, V: Send> Send for IndexedDeqeue<K, V> {}

    impl<K: PartialEq> PartialEq for IndexedDequeueKeyRef<K> {
        fn eq(&self, other: &IndexedDequeueKeyRef<K>) -> bool {
            unsafe { (*self.0).eq(&*other.0) }
        }
    }

    impl<K: PartialEq> Eq for IndexedDequeueKeyRef<K> {}

    impl<K: Hash> Hash for IndexedDequeueKeyRef<K> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            unsafe { (*self.0).hash(state) }
        }
    }

    pub struct Pool {
        entries: IndexedDeqeue<FragmentId, Fragment>,
        max_entries: usize,
    }

    impl Pool {
        pub fn new(max_entries: usize) -> Self {
            Pool {
                entries: IndexedDeqeue::new(),
                max_entries,
            }
        }

        /// Returns clones of registered fragments
        pub fn insert_all(
            &mut self,
            fragments: impl IntoIterator<Item = Fragment>,
        ) -> Vec<Fragment> {
            let max_fragments = self.max_entries - self.entries.len();
            fragments
                .into_iter()
                .filter(|fragment| {
                    let fragment_id = fragment.id();
                    if self.entries.contains(&fragment_id) {
                        false
                    } else {
                        self.entries.push_front(fragment_id, fragment.clone());
                        true
                    }
                })
                .take(max_fragments)
                .collect()
        }

        pub fn remove_all<'a>(&mut self, fragment_ids: impl IntoIterator<Item = &'a FragmentId>) {
            for fragment_id in fragment_ids {
                self.entries.remove(fragment_id);
            }
        }

        pub fn remove_oldest(&mut self) -> Option<Fragment> {
            self.entries.pop_back().map(|(_, value)| value)
        }

        pub fn return_to_pool(&mut self, fragments: impl IntoIterator<Item = Fragment>) {
            for fragment in fragments.into_iter() {
                self.entries.push_back(fragment.id(), fragment);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use quickcheck_macros::quickcheck;

        #[quickcheck]
        fn overflowing_pool_should_reject_new_fragments(
            fragments1_in: (Fragment, Fragment, Fragment),
            fragments2_in: (Fragment, Fragment),
        ) {
            let fragments1 = vec![
                fragments1_in.0.clone(),
                fragments1_in.1.clone(),
                fragments1_in.2.clone(),
            ];
            let fragments2 = vec![
                fragments1_in.2.clone(),
                fragments2_in.0.clone(),
                fragments2_in.1.clone(),
            ];
            let fragments2_expected = vec![fragments2_in.0.clone()];
            let final_expected = vec![
                fragments1_in.0,
                fragments1_in.1,
                fragments1_in.2,
                fragments2_in.0,
            ];
            let mut pool = Pool::new(4);
            assert_eq!(fragments1, pool.insert_all(fragments1.clone()));
            assert_eq!(fragments2_expected, pool.insert_all(fragments2));
            for expected in final_expected.into_iter() {
                assert_eq!(expected, pool.remove_oldest().unwrap());
            }
            assert!(pool.remove_oldest().is_none());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct_pools_number() {
        let (fake_msgbox, _) = crate::async_msg::channel(1);
        // a passive node still has 1 pool
        let pools = Pools::new(0, 0, Logs::new(1), fake_msgbox.clone(), None);
        assert_eq!(pools.pools.len(), 1);

        // a leader node should have as many pools as leaders
        let pools = Pools::new(0, 1, Logs::new(1), fake_msgbox.clone(), None);
        assert_eq!(pools.pools.len(), 1);

        let pools = Pools::new(0, 5, Logs::new(1), fake_msgbox, None);
        assert_eq!(pools.pools.len(), 5);
    }
}
