use crate::leadership::protocols::{LeaderEvent};
use jormungandr_lib::{interfaces::{BlockDate, EnclaveLeaderId}, time::SystemTime};
use serde::Serialize;
use tokio::timer::delay_queue::{self, DelayQueue};

pub struct Event {
    schedule: Schedule,

    enclave_data: LeaderEvent,
}

#[derive(Clone, Serialize)]
pub struct Schedule {
    created_at_time: SystemTime,
    scheduled_at_time: SystemTime,
    scheduled_at_date: BlockDate,
    wake_at_time: Option<SystemTime>,
    finished_at_time: Option<SystemTime>,
    enclave_leader_id: EnclaveLeaderId,
}

/// one of the main issue with the current build for the
pub struct Schedules {
    scheduled_events: DelayQueue<Event>,

    schedules: Vec<Schedule>,
}

impl Event {
    /// schedule a new leader event at the given time
    pub fn new(scheduled_at_time: SystemTime, enclave_data: LeaderEvent) -> Self {
        Event {
            schedule: Schedule::new(enclave_data.id, enclave_data.date.into(), scheduled_at_time),
            enclave_data,
        }
    }

    /// mark the current schedule is starting to process. This will add some
    /// metadata to the `Schedule` so we can later trace in the log what is
    /// happening in the schedule (especially the diff between the scheduled_at_time
    /// and wake_at_tome will give the time it actually took for schedule to start)
    pub fn mark_wake(&mut self) {
        self.schedule.mark_wake()
    }

    /// mark the current schedule has finished processing its task.
    /// it will add some metadata to follow up with when the task
    /// has finished. Allowing to extrapolate some information
    /// such as how long the schedule ran for
    pub fn mark_finished(&mut self) {
        self.schedule.mark_finished()
    }

    /// retrieve the metadata for the enclave
    pub fn leader_event(&self) -> &LeaderEvent {
        &self.enclave_data
    }
}

impl Schedule {
    fn new(
        enclave_leader_id: EnclaveLeaderId,
        scheduled_at_date: BlockDate,
        scheduled_at_time: SystemTime,
    ) -> Self {
        Schedule {
            created_at_time: SystemTime::now(),
            scheduled_at_time,
            scheduled_at_date,
            wake_at_time: None,
            finished_at_time: None,
            enclave_leader_id,
        }
    }

    fn mark_wake(&mut self) {
        debug_assert!(self.wake_at_time.is_none());
        self.wake_at_time = Some(SystemTime::now())
    }

    fn mark_finished(&mut self) {
        debug_assert!(self.finished_at_time.is_none());
        self.finished_at_time = Some(SystemTime::now())
    }
}
