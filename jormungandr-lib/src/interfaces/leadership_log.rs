use crate::{interfaces::BlockDate, time::SystemTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EnclaveLeaderId(u32);

/// log identifier in the leadership log. Can be used to update
/// back some.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeadershipLogId(EnclaveLeaderId, BlockDate);

/// provides information regarding events in the leadership schedule
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeadershipLog {
    created_at_time: SystemTime,
    scheduled_at_time: SystemTime,
    scheduled_at_date: BlockDate,
    wake_at_time: Option<SystemTime>,
    finished_at_time: Option<SystemTime>,
    enclave_leader_id: EnclaveLeaderId,
}

impl EnclaveLeaderId {
    pub fn new() -> Self {
        EnclaveLeaderId(0)
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl LeadershipLog {
    pub fn new(
        enclave_leader_id: EnclaveLeaderId,
        scheduled_at_date: BlockDate,
        scheduled_at_time: SystemTime,
    ) -> Self {
        LeadershipLog {
            created_at_time: SystemTime::now(),
            scheduled_at_time,
            scheduled_at_date,
            wake_at_time: None,
            finished_at_time: None,
            enclave_leader_id,
        }
    }

    /// retrieve a unique identifier to this log
    pub fn leadership_log_id(&self) -> LeadershipLogId {
        LeadershipLogId(self.enclave_leader_id, self.scheduled_at_date)
    }

    pub fn created_at_time(&self) -> &SystemTime {
        &self.created_at_time
    }
    pub fn scheduled_at_date(&self) -> &BlockDate {
        &self.scheduled_at_date
    }
    pub fn scheduled_at_time(&self) -> &SystemTime {
        &self.scheduled_at_time
    }
    pub fn wake_at_time(&self) -> &Option<SystemTime> {
        &self.wake_at_time
    }
    pub fn finished_at_time(&self) -> &Option<SystemTime> {
        &self.finished_at_time
    }
    pub fn enclave_leader_id(&self) -> &EnclaveLeaderId {
        &self.enclave_leader_id
    }

    /// make a leadership event as triggered.
    ///
    /// This should be called when the leadership event has started.
    ///
    /// # panic
    ///
    /// on non-release build, this function will panic if the log was already
    /// marked as awaken.
    ///
    pub fn mark_wake(&mut self) {
        debug_assert!(self.wake_at_time.is_none());
        self.wake_at_time = Some(SystemTime::now())
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
    pub fn mark_finished(&mut self) {
        debug_assert!(self.finished_at_time.is_none());
        self.finished_at_time = Some(SystemTime::now())
    }
}
