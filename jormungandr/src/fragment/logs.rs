use crate::fragment::FragmentId;
use futures03::future;
use jormungandr_lib::interfaces::{FragmentLog, FragmentStatus};
use tokio02::time::{self, Duration};

pub struct Logs(internal::Logs);

impl Logs {
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Logs(internal::Logs::new(max_entries, ttl))
    }

    /// Returns true if fragment was registered
    pub fn insert(&mut self, log: FragmentLog) -> bool {
        self.0.insert(log)
    }

    /// Returns number of registered fragments
    pub fn insert_all(&mut self, logs: impl IntoIterator<Item = FragmentLog>) -> usize {
        self.0.insert_all(logs)
    }

    pub fn exists(&self, fragment_id: FragmentId) -> bool {
        self.0.exists(&fragment_id.into())
    }

    pub fn exist_all(&self, fragment_ids: impl IntoIterator<Item = FragmentId>) -> Vec<bool> {
        let hashes = fragment_ids.into_iter().map(Into::into);
        self.0.exist_all(hashes)
    }

    pub fn modify(&mut self, fragment_id: FragmentId, status: FragmentStatus) {
        self.0.modify(&fragment_id.into(), status);
    }

    pub fn modify_all(
        &mut self,
        fragment_ids: impl IntoIterator<Item = FragmentId>,
        status: FragmentStatus,
    ) {
        for fragment_id in fragment_ids {
            let id = fragment_id.into();
            self.0.modify(&id, status.clone())
        }
    }

    pub async fn poll_purge(&mut self) -> Result<(), time::Error> {
        future::poll_fn(|cx| self.0.poll_purge(cx)).await
    }

    pub fn logs(&self) -> Vec<FragmentLog> {
        self.0.logs().cloned().collect()
    }

    pub(super) fn inner(&mut self) -> &mut internal::Logs {
        &mut self.0
    }
}

pub(super) mod internal {
    use futures03::{
        stream::Stream,
        task::{Context, Poll},
    };
    use jormungandr_lib::{
        crypto::hash::Hash,
        interfaces::{FragmentLog, FragmentOrigin, FragmentStatus},
    };
    use std::{
        collections::hash_map::{Entry, HashMap},
        pin::Pin,
        time::Duration,
    };
    use tokio02::time::{self, delay_queue, DelayQueue, Instant};

    pub struct Logs {
        max_entries: usize,
        entries: HashMap<Hash, (FragmentLog, delay_queue::Key)>,
        expirations: Pin<Box<DelayQueue<Hash>>>,
        ttl: Duration,
    }

    impl Logs {
        pub fn new(max_entries: usize, ttl: Duration) -> Self {
            Logs {
                max_entries,
                entries: HashMap::new(),
                expirations: Box::pin(DelayQueue::new()),
                ttl,
            }
        }

        pub fn exists(&self, fragment_id: &Hash) -> bool {
            self.entries.contains_key(fragment_id)
        }

        pub fn exist_all(&self, fragment_ids: impl IntoIterator<Item = Hash>) -> Vec<bool> {
            fragment_ids
                .into_iter()
                .map(|fragment_id| self.exists(&fragment_id))
                .collect()
        }

        /// Returns true if fragment was registered
        pub fn insert(&mut self, log: FragmentLog) -> bool {
            if self.max_entries < self.entries.len() {
                false
            } else {
                let fragment_id = *log.fragment_id();
                let entry = match self.entries.entry(fragment_id) {
                    Entry::Occupied(_) => return false,
                    Entry::Vacant(entry) => entry,
                };
                let delay = self.expirations.insert(fragment_id, self.ttl);
                entry.insert((log, delay));
                true
            }
        }

        /// Returns number of registered fragments
        pub fn insert_all(&mut self, logs: impl IntoIterator<Item = FragmentLog>) -> usize {
            logs.into_iter()
                .take(
                    self.max_entries
                        .checked_sub(self.entries.len())
                        .unwrap_or(0),
                )
                .map(|log| self.insert(log))
                .filter(|was_modified| *was_modified)
                .count()
        }

        pub fn modify(&mut self, fragment_id: &Hash, status: FragmentStatus) {
            let len = self.entries.len();
            match self.entries.entry(fragment_id.clone()) {
                Entry::Occupied(mut entry) => {
                    entry.get_mut().0.modify(status);

                    self.expirations
                        .reset_at(&entry.get().1, Instant::now() + self.ttl);
                }
                Entry::Vacant(entry) => {
                    // while a log modification, if the log was not already present in the
                    // logs it means we received it from the a new block from the network.
                    // we can mark the status of the transaction so newly received transaction
                    // be stored.

                    if self.max_entries < len {
                        let delay = self.expirations.insert(*fragment_id, self.ttl);
                        entry.insert((
                            FragmentLog::new(
                                fragment_id.clone().into_hash(),
                                FragmentOrigin::Network,
                            ),
                            delay,
                        ));
                    }
                }
            }
        }

        pub fn poll_purge(&mut self, cx: &mut Context) -> Poll<Result<(), time::Error>> {
            loop {
                match self.expirations.as_mut().poll_next(cx) {
                    Poll::Ready(Some(Ok(entry))) => {
                        self.entries.remove(entry.get_ref());
                    }
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                    Poll::Ready(None) => return Poll::Ready(Ok(())),

                    // Here Pending means there are still items in the DelayQueue but
                    // they are not expired. We don't want this function to wait for these
                    // ones to expired. We only cared about removing the expired ones.
                    Poll::Pending => return Poll::Ready(Ok(())),
                }
            }
        }

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a FragmentLog> {
            self.entries.values().map(|(v, _)| v)
        }
    }
}
