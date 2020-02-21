pub use jormungandr_lib::interfaces::LeadershipLogStatus;
use jormungandr_lib::interfaces::{LeadershipLog, LeadershipLogId};
use std::sync::Arc;
use tokio02::sync::RwLock;

/// all leadership logs, allow for following up on the different entity
/// of the blockchain
#[derive(Clone)]
pub struct Logs(Arc<RwLock<internal::Logs>>);

/// leadership log handle. will allow to update the status of the log
/// without having to hold the [`Logs`]
///
/// [`Logs`]: ./struct.Logs.html
#[derive(Clone)]
pub struct LeadershipLogHandle {
    internal_id: LeadershipLogId,
    logs: Logs,
}

impl LeadershipLogHandle {
    /// make a leadership event as triggered.
    ///
    /// This should be called when the leadership event has started.
    ///
    /// # panic
    ///
    /// on non-release build, this function will panic if the log was already
    /// marked as awaken.
    ///
    pub async fn mark_wake(&self) {
        self.logs.mark_wake(self.internal_id).await
    }

    pub async fn set_status(&self, status: LeadershipLogStatus) {
        self.logs.set_status(self.internal_id, status).await
    }

    /// make a leadership event as finished.
    ///
    /// This should be called when the leadership event has finished its
    /// scheduled action.
    ///
    /// # panic
    ///
    /// on non-release build, this function will panic if the log was already
    /// marked as finished.
    ///
    pub async fn mark_finished(&self) {
        self.logs.mark_finished(self.internal_id).await
    }
}

impl Logs {
    /// create a Leadership Logs. Logs will be removed once the `Logs` passed
    /// beyond a certain number of entries.
    ///
    pub fn new(cap: usize) -> Self {
        Logs(Arc::new(RwLock::new(internal::Logs::new(cap))))
    }

    pub async fn insert(&self, log: LeadershipLog) -> Result<LeadershipLogHandle, ()> {
        let logs = self.clone();
        let id = logs.0.write().await.insert(log);
        Ok(LeadershipLogHandle {
            internal_id: id,
            logs: logs,
        })
    }

    async fn mark_wake(&self, leadership_log_id: LeadershipLogId) {
        let inner = self.0.clone();
        inner.write().await.mark_wake(&leadership_log_id.into());
    }

    async fn set_status(&self, leadership_log_id: LeadershipLogId, status: LeadershipLogStatus) {
        let inner = self.0.clone();
        inner
            .write()
            .await
            .set_status(&leadership_log_id.into(), status);
    }

    async fn mark_finished(&self, leadership_log_id: LeadershipLogId) {
        let inner = self.0.clone();
        inner.write().await.mark_finished(&leadership_log_id.into());
    }

    pub async fn logs(&self) -> Vec<LeadershipLog> {
        let inner = self.0.clone();
        let guard = inner.read().await;
        guard.logs().cloned().collect()
    }
}

pub(super) mod internal {
    use super::{LeadershipLog, LeadershipLogId, LeadershipLogStatus};
    use lru::LruCache;

    pub struct Logs {
        entries: LruCache<LeadershipLogId, LeadershipLog>,
    }

    impl Logs {
        pub fn new(cap: usize) -> Self {
            Logs {
                entries: LruCache::new(cap),
            }
        }

        pub fn insert(&mut self, log: LeadershipLog) -> LeadershipLogId {
            let id = log.leadership_log_id();

            self.entries.put(id, log);
            id
        }

        pub fn mark_wake(&mut self, leadership_log_id: &LeadershipLogId) {
            if let Some(ref mut log) = self.entries.get_mut(leadership_log_id) {
                log.mark_wake();
            }
        }

        pub fn set_status(
            &mut self,
            leadership_log_id: &LeadershipLogId,
            status: LeadershipLogStatus,
        ) {
            if let Some(ref mut log) = self.entries.get_mut(leadership_log_id) {
                log.set_status(status);
            }
        }

        pub fn mark_finished(&mut self, leadership_log_id: &LeadershipLogId) {
            if let Some(ref mut log) = self.entries.get_mut(leadership_log_id) {
                log.mark_finished();
            }
        }

        pub fn logs<'a>(&'a self) -> impl Iterator<Item = &'a LeadershipLog> {
            self.entries.iter().map(|(_, v)| v)
        }
    }
}
