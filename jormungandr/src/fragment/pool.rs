use crate::{
    blockcfg::ApplyBlockLedger,
    blockchain::{Ref, Tip},
    fragment::{
        selection::{
            FragmentSelectionAlgorithm, FragmentSelectionAlgorithmParams, FragmentSelectionResult,
            OldestFirst,
        },
        Fragment, FragmentId, Logs,
    },
    intercom::{NetworkMsg, PropagateMsg},
    metrics::{Metrics, MetricsBackend},
    utils::async_msg::MessageBox,
};
use chain_core::{packer::Codec, property::Serialize};
use chain_impl_mockchain::{
    block::BlockDate, fragment::Contents, setting::Settings, transaction::Transaction,
};
use futures::{channel::mpsc::SendError, sink::SinkExt};
use jormungandr_lib::{
    interfaces::{
        BlockDate as BlockDateDto, FragmentLog, FragmentOrigin, FragmentRejectionReason,
        FragmentStatus, FragmentsProcessingSummary, PersistentFragmentLog, RejectedFragmentInfo,
    },
    time::SecondsSinceUnixEpoch,
};
use std::mem;
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
};
use tracing::Instrument;

// It's a pretty big buffer, but common cloud based storage solutions (like EBS or GlusterFS) benefits from
// this and it's currently flushed after every request, so the possibility of losing fragments due to a crash
// should be minimal.
// Its main purpose is to avoid unnecessary flushing while processing a single batch of fragments.
const DEFAULT_BUF_SIZE: usize = 128 * 1024; // 128 KiB

pub struct Pool {
    logs: Logs,
    pool: internal::Pool,
    network_msg_box: MessageBox<NetworkMsg>,
    persistent_log: Option<BufWriter<File>>,
    tip: Tip,
    metrics: Metrics,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot propagate a fragment to the network")]
    CannotPropagate(#[source] SendError),
}

impl Pool {
    pub fn new(
        max_entries: usize,
        logs: Logs,
        network_msg_box: MessageBox<NetworkMsg>,
        persistent_log: Option<File>,
        tip: Tip,
        metrics: Metrics,
    ) -> Self {
        Pool {
            logs,
            pool: internal::Pool::new(max_entries),
            network_msg_box,
            persistent_log: persistent_log
                .map(|file| BufWriter::with_capacity(DEFAULT_BUF_SIZE, file)),
            tip,
            metrics,
        }
    }

    pub fn logs(&mut self) -> &mut Logs {
        &mut self.logs
    }

    /// Sets the persistent log to a file.
    /// The file must be opened for writing.
    pub fn set_persistent_log(&mut self, file: File) {
        self.persistent_log = Some(BufWriter::with_capacity(DEFAULT_BUF_SIZE, file));
    }

    /// Synchronizes the persistent log file contents and metadata
    /// to the file system and closes the file.
    pub async fn close_persistent_log(&mut self) {
        if let Some(mut persistent_log) = mem::replace(&mut self.persistent_log, None) {
            if let Err(error) = persistent_log.flush().await {
                tracing::error!(%error, "failed to flush persistent log");
            }
            if let Err(error) = persistent_log.into_inner().sync_all().await {
                tracing::error!(%error, "failed to sync persistent log file");
            }
        }
    }

    async fn filter_fragment(
        &mut self,
        fragment: &Fragment,
        id: FragmentId,
        ledger_settings: &Settings,
        block_date: BlockDate,
    ) -> Result<(), FragmentRejectionReason> {
        if self.logs.exists(id) {
            tracing::debug!("fragment is already logged");
            return Err(FragmentRejectionReason::FragmentAlreadyInLog);
        }

        if let Some(valid_until) = get_transaction_expiry_date(fragment) {
            use chain_impl_mockchain::ledger::check::{valid_transaction_date, TxValidityError};
            match valid_transaction_date(ledger_settings, valid_until, block_date) {
                Ok(_) => {}
                Err(TxValidityError::TransactionExpired) => {
                    tracing::debug!("fragment is expired at the time of receiving");
                    return Err(FragmentRejectionReason::FragmentExpired);
                }
                Err(TxValidityError::TransactionValidForTooLong) => {
                    tracing::debug!("fragment is valid for too long");
                    return Err(FragmentRejectionReason::FragmentValidForTooLong);
                }
            }
        }

        if !is_fragment_valid(fragment) {
            tracing::debug!("fragment is invalid, not including to the pool");
            return Err(FragmentRejectionReason::FragmentInvalid);
        }

        if let Some(persistent_log) = self.persistent_log.as_mut() {
            let entry = PersistentFragmentLog {
                time: SecondsSinceUnixEpoch::now(),
                fragment: fragment.clone(),
            };
            // this must be sufficient: the PersistentFragmentLog format is using byte array
            // for serialization so we do not expect any problems during deserialization
            let mut codec = Codec::new(Vec::new());
            entry.serialize(&mut codec).unwrap();
            if let Err(err) = persistent_log
                .write_all(codec.into_inner().as_slice())
                .await
            {
                tracing::error!(err = %err, "failed to write persistent fragment log entry");
            }
        }

        tracing::debug!("including fragment to the pool");
        Ok(())
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

        let mut filtered_fragments = Vec::new();
        let mut rejected = Vec::new();

        let mut fragments = fragments.into_iter().map(|el| {
            let id = el.hash();
            (el, id)
        });

        let tip = self.tip.get_ref().await;
        let ledger = tip.ledger();
        let ledger_settings = ledger.settings();
        let block_date = get_current_block_date(&tip);

        for (fragment, id) in fragments.by_ref() {
            let span = tracing::debug_span!("pool_incoming_fragment", fragment_id=?id);

            match self
                .filter_fragment(&fragment, id, ledger_settings, block_date)
                .instrument(span)
                .await
            {
                Err(reason @ FragmentRejectionReason::FragmentInvalid) => {
                    rejected.push(RejectedFragmentInfo { id, reason });
                    if fail_fast {
                        tracing::debug!("fail_fast is enabled; rejecting all downstream fragments");
                        break;
                    }
                }
                Err(reason) => rejected.push(RejectedFragmentInfo { id, reason }),
                Ok(()) => filtered_fragments.push((fragment, id)),
            }
        }

        // flush every request to minimize possibility of losing fragments at the expense of non optimal performance
        if let Some(persistent_log) = self.persistent_log.as_mut() {
            if let Err(error) = persistent_log.flush().await {
                tracing::error!(%error, "failed to flush persistent logs");
            }
        }

        if fail_fast {
            for (_, id) in fragments {
                tracing::error!(
                    %id, "rejected due to fail_fast and one of previous fragments being invalid"
                );
                rejected.push(RejectedFragmentInfo {
                    id,
                    reason: FragmentRejectionReason::PreviousFragmentInvalid,
                })
            }
        }

        let span = tracing::trace_span!("pool_insert_fragment");
        let _enter = span.enter();

        let mut fragments = filtered_fragments.into_iter();
        let new_fragments = self.pool.insert_all(fragments.by_ref());
        let count = new_fragments.len();
        tracing::debug!("{} of the received fragments were added to the pool", count);
        let fragment_logs: Vec<_> = new_fragments
            .iter()
            .map(move |(_, id)| FragmentLog::new(*id, origin))
            .collect();
        self.logs.insert_all_pending(fragment_logs);

        self.update_metrics();

        let mut accepted = Vec::new();
        let mut network_msg_box = self.network_msg_box.clone();
        for (fragment, id) in new_fragments {
            tracing::debug!(fragment_id=?id, "inserted fragment to the pool");
            accepted.push(id);
            let fragment_msg = NetworkMsg::Propagate(Box::new(PropagateMsg::Fragment(fragment)));
            network_msg_box
                .send(fragment_msg)
                .await
                .map_err(Error::CannotPropagate)?;
        }

        for (_, id) in fragments {
            tracing::debug!(fragment_id=?id, "rejecting fragment due to pool overflow");
            rejected.push(RejectedFragmentInfo {
                id,
                reason: FragmentRejectionReason::PoolOverflow,
            });
        }

        Ok(FragmentsProcessingSummary { accepted, rejected })
    }

    pub fn remove_added_to_block(&mut self, fragment_ids: Vec<FragmentId>, status: FragmentStatus) {
        let date = if let FragmentStatus::InABlock { date, .. } = status {
            date
        } else {
            panic!("expected status to be in block, found {:?}", status);
        };
        self.pool.remove_all(fragment_ids.iter());
        self.logs.modify_all(fragment_ids, status, date);
        self.update_metrics();
    }

    pub async fn select(
        &mut self,
        ledger: ApplyBlockLedger,
        selection_alg: FragmentSelectionAlgorithmParams,
        soft_deadline_future: futures::channel::oneshot::Receiver<()>,
        hard_deadline_future: futures::channel::oneshot::Receiver<()>,
    ) -> (Contents, ApplyBlockLedger) {
        let Pool { logs, pool, .. } = self;
        let FragmentSelectionResult {
            contents,
            ledger,
            rejected_fragments_cnt,
        } = match selection_alg {
            FragmentSelectionAlgorithmParams::OldestFirst => {
                let mut selection_alg = OldestFirst::new();
                selection_alg
                    .select(
                        ledger,
                        logs,
                        pool,
                        soft_deadline_future,
                        hard_deadline_future,
                    )
                    .await
            }
        };
        self.metrics.add_tx_rejected_cnt(rejected_fragments_cnt);
        self.update_metrics();
        (contents, ledger)
    }

    // Remove from logs fragments that were confirmed (or rejected) in a branch
    pub fn prune_after_ledger_branch(&mut self, branch_date: BlockDateDto) {
        self.logs.remove_logs_after_date(branch_date);
        self.update_metrics();
    }

    pub async fn remove_expired_txs(&mut self) {
        let tip = self.tip.get_ref().await;
        let block_date = get_current_block_date(&tip);
        let fragment_ids = self.pool.remove_expired_txs(block_date);
        self.metrics.add_tx_rejected_cnt(fragment_ids.len());
        self.logs.modify_all(
            fragment_ids,
            FragmentStatus::Rejected {
                reason: "fragment expired".to_string(),
            },
            block_date.into(),
        );
        self.update_metrics();
    }

    fn update_metrics(&self) {
        let mempool_usage_ratio = match self.pool.max_entries() {
            // a little arbitrary, but you could say the mempool is indeed full and it
            // does not required any special logic somewhere else
            0 => 1.0,
            n => self.pool.len() as f64 / n as f64,
        };
        self.metrics.set_mempool_usage_ratio(mempool_usage_ratio);
        self.metrics
            .set_mempool_total_size(self.pool.total_size_bytes());
    }
}

fn get_current_block_date(tip: &Ref) -> BlockDate {
    let time = std::time::SystemTime::now();
    let era = tip.epoch_leadership_schedule().era();
    let epoch_position = tip
        .time_frame()
        .slot_at(&time)
        .and_then(|slot| era.from_slot_to_era(slot))
        .expect("the current time and blockchain state should produce a valid blockchain date");
    let block_date: BlockDate = epoch_position.into();
    BlockDate {
        slot_id: block_date.slot_id + 1,
        ..block_date
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
        Fragment::UpdateProposal(ref tx) => is_transaction_valid(tx),
        Fragment::UpdateVote(ref tx) => is_transaction_valid(tx),
        Fragment::VotePlan(ref tx) => is_transaction_valid(tx),
        Fragment::VoteCast(ref tx) => is_transaction_valid(tx),
        Fragment::VoteTally(ref tx) => is_transaction_valid(tx),
        Fragment::MintToken(ref tx) => is_transaction_valid(tx),
        // evm stuff
        // TODO, maybe we need to develop some evm specific stateless validation in this place
        Fragment::Evm(_) => true,
        Fragment::EvmMapping(ref tx) => is_transaction_valid(tx),
    }
}

fn is_transaction_valid<E>(tx: &Transaction<E>) -> bool {
    tx.verify_possibly_balanced().is_ok()
}

fn get_transaction_expiry_date(fragment: &Fragment) -> Option<BlockDate> {
    match fragment {
        Fragment::Initial(_) => None,
        Fragment::OldUtxoDeclaration(_) => None,
        Fragment::Evm(_) => None,
        Fragment::Transaction(tx) => Some(tx.as_slice().valid_until()),
        Fragment::OwnerStakeDelegation(tx) => Some(tx.as_slice().valid_until()),
        Fragment::StakeDelegation(tx) => Some(tx.as_slice().valid_until()),
        Fragment::PoolRegistration(tx) => Some(tx.as_slice().valid_until()),
        Fragment::PoolRetirement(tx) => Some(tx.as_slice().valid_until()),
        Fragment::PoolUpdate(tx) => Some(tx.as_slice().valid_until()),
        Fragment::UpdateProposal(tx) => Some(tx.as_slice().valid_until()),
        Fragment::UpdateVote(tx) => Some(tx.as_slice().valid_until()),
        Fragment::VotePlan(tx) => Some(tx.as_slice().valid_until()),
        Fragment::VoteCast(tx) => Some(tx.as_slice().valid_until()),
        Fragment::VoteTally(tx) => Some(tx.as_slice().valid_until()),
        Fragment::MintToken(tx) => Some(tx.as_slice().valid_until()),
        Fragment::EvmMapping(tx) => Some(tx.as_slice().valid_until()),
    }
}

pub(super) mod internal {
    use super::*;
    use std::{
        cmp::Ordering,
        collections::{BTreeSet, HashMap},
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
    unsafe impl<K: Sync, V: Sync> Sync for IndexedDeqeue<K, V> {}

    unsafe impl<K: Send, V: Send> Send for IndexedDequeueEntry<K, V> {}
    unsafe impl<K: Send> Send for IndexedDequeueKeyRef<K> {}

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

    #[derive(Clone, PartialEq, Eq)]
    struct TimeoutQueueItem {
        valid_until: BlockDate,
        id: FragmentId,
    }

    impl Ord for TimeoutQueueItem {
        fn cmp(&self, other: &Self) -> Ordering {
            let res = self.valid_until.cmp(&other.valid_until);
            if res != Ordering::Equal {
                return res;
            }
            self.id.cmp(&other.id)
        }
    }

    impl PartialOrd for TimeoutQueueItem {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    pub struct Pool {
        entries: IndexedDeqeue<FragmentId, Fragment>,
        timeout_queue: BTreeSet<TimeoutQueueItem>,
        max_entries: usize,
        total_size_bytes: usize,
    }

    impl Pool {
        pub fn new(max_entries: usize) -> Self {
            Pool {
                entries: IndexedDeqeue::new(),
                // Using BTreeSet is a nasty hack so that we are able to to efficiently remove items
                // out of their order in a queue. BinaryHeap does not allow that.
                timeout_queue: BTreeSet::new(),
                max_entries,
                total_size_bytes: 0,
            }
        }

        /// Returns clones of registered fragments
        pub fn insert_all(
            &mut self,
            fragments: impl IntoIterator<Item = (Fragment, FragmentId)>,
        ) -> Vec<(Fragment, FragmentId)> {
            let max_fragments = self.max_entries - self.entries.len();
            fragments
                .into_iter()
                .filter(|(fragment, id)| {
                    if self.entries.contains(id) {
                        false
                    } else {
                        self.total_size_bytes += fragment.serialized_size();
                        self.timeout_queue_insert(fragment, *id);
                        self.entries.push_front(*id, fragment.clone());
                        true
                    }
                })
                .take(max_fragments)
                .collect()
        }

        pub fn remove_all<'a>(&mut self, fragment_ids: impl IntoIterator<Item = &'a FragmentId>) {
            for fragment_id in fragment_ids {
                let maybe_fragment = self.entries.remove(fragment_id);
                if let Some(fragment) = maybe_fragment {
                    self.timeout_queue_remove(&fragment, *fragment_id);
                    self.total_size_bytes -= fragment.serialized_size();
                }
            }
        }

        pub fn remove_oldest(&mut self) -> Option<(Fragment, FragmentId)> {
            let (id, fragment) = self.entries.pop_back().map(|(id, value)| (id, value))?;
            self.timeout_queue_remove(&fragment, id);
            self.total_size_bytes -= fragment.serialized_size();
            Some((fragment, id))
        }

        pub fn return_to_pool(
            &mut self,
            fragments: impl IntoIterator<Item = (Fragment, FragmentId)>,
        ) {
            for (fragment, id) in fragments.into_iter() {
                self.timeout_queue_insert(&fragment, id);
                self.total_size_bytes += fragment.serialized_size();
                self.entries.push_back(id, fragment);
            }
        }

        fn timeout_queue_insert(&mut self, fragment: &Fragment, id: FragmentId) {
            if let Some(valid_until) = get_transaction_expiry_date(fragment) {
                let item = TimeoutQueueItem { valid_until, id };
                self.timeout_queue.insert(item);
            }
        }

        fn timeout_queue_remove(&mut self, fragment: &Fragment, id: FragmentId) {
            if let Some(valid_until) = get_transaction_expiry_date(fragment) {
                let item = TimeoutQueueItem { valid_until, id };
                self.timeout_queue.remove(&item);
            }
        }

        pub fn remove_expired_txs(&mut self, block_date: BlockDate) -> Vec<FragmentId> {
            let to_remove: Vec<_> = self
                .timeout_queue
                .iter()
                .take_while(|x| x.valid_until < block_date)
                .cloned()
                .collect();
            for item in &to_remove {
                self.timeout_queue.remove(item);
                if let Some(fragment) = self.entries.remove(&item.id) {
                    self.total_size_bytes -= fragment.serialized_size();
                }
            }
            to_remove.into_iter().map(|x| x.id).collect()
            // TODO convert to something like this when .first() and .pop_first() are stabilized. This does not have unnecessary clones.
            // https://github.com/rust-lang/rust/issues/62924
            // loop {
            //     if let Some(item) = self.timeout_queue.first() {
            //         if item.valid_until < block_date {
            //             break;
            //         }
            //     } else {
            //         break;
            //     }

            //     let item = self.timeout_queue.pop_first().unwrap();
            //     self.entries.remove(&item.id);
            // }
        }

        pub fn len(&self) -> usize {
            self.entries.len()
        }

        pub fn total_size_bytes(&self) -> usize {
            self.total_size_bytes
        }

        pub fn max_entries(&self) -> usize {
            self.max_entries
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use chain_core::property::Fragment as _;
        use chain_impl_mockchain::transaction::TxBuilder;
        use quickcheck::TestResult;
        use quickcheck_macros::quickcheck;
        use std::collections::HashSet;

        #[quickcheck]
        fn overflowing_pool_should_reject_new_fragments(
            fragments1_in: (Fragment, Fragment, Fragment),
            fragments2_in: (Fragment, Fragment),
        ) -> TestResult {
            let fragments1 = vec![
                (fragments1_in.0.clone(), fragments1_in.0.id()),
                (fragments1_in.1.clone(), fragments1_in.1.id()),
                (fragments1_in.2.clone(), fragments1_in.2.id()),
            ];
            let fragments2 = vec![
                (fragments1_in.2.clone(), fragments1_in.2.id()),
                (fragments2_in.0.clone(), fragments2_in.0.id()),
                (fragments2_in.1.clone(), fragments2_in.1.id()),
            ];

            if fragments1
                .iter()
                .chain(fragments2.iter())
                .map(|(_, id)| id)
                .collect::<HashSet<_>>()
                .len()
                != 5
            {
                return TestResult::discard();
            }

            let fragments2_expected = vec![(fragments2_in.0.clone(), fragments2_in.0.id())];
            let final_expected = vec![
                (fragments1_in.0.clone(), fragments1_in.0.id()),
                (fragments1_in.1.clone(), fragments1_in.1.id()),
                (fragments1_in.2.clone(), fragments1_in.2.id()),
                (fragments2_in.0.clone(), fragments2_in.0.id()),
            ];
            let mut pool = Pool::new(4);
            assert_eq!(fragments1, pool.insert_all(fragments1.clone()));
            assert_eq!(
                pool.total_size_bytes,
                fragments1
                    .iter()
                    .map(|(f, _)| f.serialized_size())
                    .sum::<usize>()
            );

            assert_eq!(fragments2_expected, pool.insert_all(fragments2));
            for expected in final_expected.into_iter() {
                assert_eq!(expected, pool.remove_oldest().unwrap());
            }
            TestResult::from_bool(pool.remove_oldest().is_none())
        }

        #[test]
        fn expired_transactions_are_removed() {
            let mut pool = Pool::new(1);

            let tx = Fragment::Transaction(
                TxBuilder::new()
                    .set_nopayload()
                    .set_expiry_date(BlockDate {
                        epoch: 0,
                        slot_id: 1,
                    })
                    .set_ios(&[], &[])
                    .set_witnesses(&[])
                    .set_payload_auth(&()),
            );

            pool.insert_all([(tx.clone(), tx.id())]);

            assert_eq!(pool.entries.len(), 1, "Fragment should be in pool");

            pool.remove_expired_txs(BlockDate {
                epoch: 0,
                slot_id: 1,
            });

            assert_eq!(pool.entries.len(), 1, "Fragment has not expired yet");

            pool.remove_expired_txs(BlockDate {
                epoch: 0,
                slot_id: 2,
            });

            assert_eq!(pool.entries.len(), 0, "Expired fragment should be removed");
        }
    }
}
