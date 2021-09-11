use crate::fragment::FragmentId;
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{BlockDate, FragmentLog, FragmentOrigin, FragmentStatus},
};
use lru::LruCache;
use std::collections::HashMap;

pub struct Logs {
    entries: LruCache<Hash, (FragmentLog, Option<BlockDate>)>,
}

impl Logs {
    pub fn new(max_entries: usize) -> Self {
        Logs {
            entries: LruCache::new(max_entries),
        }
    }

    pub fn exists(&self, fragment_id: FragmentId) -> bool {
        let fragment_id: Hash = fragment_id.into();
        self.entries.contains(&fragment_id)
    }

    pub fn exist_all(&self, fragment_ids: impl IntoIterator<Item = FragmentId>) -> Vec<bool> {
        fragment_ids
            .into_iter()
            .map(|fragment_id| self.exists(fragment_id))
            .collect()
    }

    /// Returns true if fragment was registered
    pub fn insert_pending(&mut self, log: FragmentLog) -> bool {
        assert!(log.is_pending());
        let fragment_id = *log.fragment_id();

        if self.entries.contains(&fragment_id) {
            false
        } else {
            self.entries.put(fragment_id, (log, None));

            true
        }
    }

    /// Returns number of registered fragments
    pub fn insert_all_pending(&mut self, logs: impl IntoIterator<Item = FragmentLog>) -> usize {
        logs.into_iter()
            .map(|log| self.insert_pending(log))
            .filter(|was_modified| *was_modified)
            .count()
    }

    pub fn modify(
        &mut self,
        fragment_id: FragmentId,
        status: FragmentStatus,
        ledger_date: BlockDate,
    ) {
        let fragment_id: Hash = fragment_id.into();
        match self.entries.get_mut(&fragment_id) {
            Some((entry, date)) => {
                if !entry.modify(status) {
                    tracing::debug!("the fragment log update was refused: cannot mark the fragment as invalid if it was already committed to a block");
                } else {
                    *date = Some(ledger_date);
                }
            }
            None => {
                // Possible reasons for entering this branch are:
                //
                // - Receiving a fragment with a network block.
                // - Having a fragment evicted from the log due to overflow.
                //
                // For both scenarios the code defaults to FragmentOrigin::Network, since there are
                // no means of knowing where the fragment came from.
                //
                // Also, in this scenario we accept any provided FragmentStatus, since we do not
                // actually know what the previous status was, and thus cannot execute the correct
                // state transition.
                let mut entry = FragmentLog::new(fragment_id.into_hash(), FragmentOrigin::Network);
                entry.modify(status);
                self.entries.put(fragment_id, (entry, Some(ledger_date)));
            }
        }
    }

    pub fn modify_all(
        &mut self,
        fragment_ids: impl IntoIterator<Item = FragmentId>,
        status: FragmentStatus,
        ledger_date: BlockDate,
    ) {
        for fragment_id in fragment_ids {
            self.modify(fragment_id, status.clone(), ledger_date);
        }
    }

    pub fn logs_by_ids(
        &self,
        fragment_ids: impl IntoIterator<Item = FragmentId>,
    ) -> HashMap<FragmentId, &FragmentLog> {
        let mut result = HashMap::new();
        fragment_ids
            .into_iter()
            .filter_map(|fragment_id| {
                let key: Hash = fragment_id.into();
                self.entries.peek(&key).map(|log| (fragment_id, log))
            })
            .for_each(|(k, (log, _date))| {
                result.insert(k, log);
            });
        result
    }

    pub fn logs(&self) -> impl Iterator<Item = &FragmentLog> {
        self.entries.iter().map(|(_, (log, _date))| log)
    }

    pub fn remove_logs_after_date(&mut self, target_date: BlockDate) {
        let mut to_remove = Vec::new();
        for (_, (log, date)) in self.entries.iter() {
            match log.status() {
                FragmentStatus::InABlock { .. } | FragmentStatus::Rejected { .. } => {
                    // date is always present for non pending statuses.
                    if date.unwrap() > target_date {
                        to_remove.push(*log.fragment_id());
                    } else {
                        // iterating in most-recently used order (i.e. most recently added to block)
                        break;
                    }
                }
                FragmentStatus::Pending => (),
            }
        }

        for fragment in to_remove {
            self.entries.pop(&fragment);
        }
    }
}
