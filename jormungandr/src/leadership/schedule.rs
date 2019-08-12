use crate::{
    blockcfg::{Leadership, LedgerParameters},
    leadership::{LeaderEvent, LeadershipLogHandle, Logs},
};
use jormungandr_lib::{interfaces::LeadershipLog, time::SystemTime};
use std::sync::Arc;
use tokio::{
    prelude::*,
    timer::delay_queue::{self, DelayQueue},
};

pub struct Schedule {
    /// keep a hand on the log handle so we can update
    /// the logs as we see fit
    pub(super) log: LeadershipLogHandle,

    /// data for the enclave to work on
    pub(super) leader_event: LeaderEvent,

    /// leadership valid for the ongoing epoch
    pub(super) leadership: Arc<Leadership>,

    /// parameters valid for the on going epochs
    pub(super) epoch_ledger_parameters: Arc<LedgerParameters>,
}

/// one of the main issue with the current build for the
pub struct Schedules {
    scheduler: DelayQueue<Schedule>,
}

impl Schedule {
    pub fn log_handle(&self) -> &LeadershipLogHandle {
        &self.log
    }

    pub fn leader_event(&self) -> &LeaderEvent {
        &self.leader_event
    }

    pub fn leadership(&self) -> &Arc<Leadership> {
        &self.leadership
    }

    pub fn ledger_parameters(&self) -> &Arc<LedgerParameters> {
        &self.epoch_ledger_parameters
    }
}

impl Schedules {
    pub fn new() -> Self {
        Schedules {
            scheduler: DelayQueue::new(),
        }
    }

    pub fn schedule(
        mut self,
        logs: Logs,
        leadership: Arc<Leadership>,
        epoch_ledger_parameters: Arc<LedgerParameters>,
        scheduled_at_time: SystemTime,
        leader_event: LeaderEvent,
    ) -> impl Future<Item = Self, Error = ()> {
        let now = std::time::Instant::now();
        let duration = scheduled_at_time
            .as_ref()
            .duration_since(std::time::SystemTime::now())
            .unwrap();
        let scheduled_time = now + duration;

        let log = LeadershipLog::new(leader_event.id, leader_event.date.into(), scheduled_at_time);
        logs.insert(log)
            .map(move |handle| Schedule {
                log: handle,
                leadership,
                epoch_ledger_parameters,
                leader_event,
            })
            .map(move |schedule| {
                self.scheduler.insert_at(schedule, scheduled_time);
                self
            })
    }
}

impl Stream for Schedules {
    type Error = tokio::timer::Error;
    type Item = delay_queue::Expired<Schedule>;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match try_ready!(self.scheduler.poll()) {
            Some(item) => Ok(Async::Ready(Some(item))),
            None => Ok(Async::NotReady),
        }
    }
}
