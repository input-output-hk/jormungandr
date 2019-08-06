use crate::leadership::protocols::{LeadershipLogHandle, Logs, LeaderEvent};
use jormungandr_lib::{time::SystemTime, interfaces::LeadershipLog};
use tokio::{prelude::*, timer::delay_queue::{self, DelayQueue}, sync::lock::Lock};

pub struct Schedule {
    /// keep a hand on the log handle so we can update
    /// the logs as we see fit
    log: LeadershipLogHandle,

    /// data for the enclave to work on
    leader_event: LeaderEvent,
}

/// one of the main issue with the current build for the
pub struct Schedules {
    scheduler: Lock<DelayQueue<Schedule>>,

    logs: Logs
}

impl Schedules {
    pub fn new(logger: Logs) -> Self {
        Schedules {
            scheduler: Lock::new(DelayQueue::new()),
            logs: logger,
        }
    }

    pub fn schedule(&mut self, scheduled_at_time: SystemTime, leader_event: LeaderEvent) -> impl Future<Item = (), Error = ()> {
        let log = LeadershipLog::new(leader_event.id, leader_event.date.into(), scheduled_at_time);
        let scheduler = self.scheduler.clone();
        self.logs.insert(log)
            .map(move |handle| {
                Schedule {
                    log: handle,
                    leader_event,
                }
            })
            .and_then(|schedule| {
                scheduler.insert_at(schedule, std::time::Instant::now())
            })
    }
}