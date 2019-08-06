use crate::{
    leadership::TaskParameters,
    secure::enclave::{Enclave, LeaderEvent},
};
use jormungandr_lib::interfaces::EnclaveLeaderId as LeaderId;
use chain_time::era::{EpochPosition, EpochSlotOffset};
use slog::Logger;
use std::time::SystemTime;
use tokio::{
    prelude::*,
    timer::{delay_queue::Expired, DelayQueue},
};

/// structure to prepare the schedule of a leader
///
/// This object will generate a steam of events at precise times
/// where the `Leader` is expected to create a block.
pub struct LeaderSchedule {
    events: DelayQueue<ScheduledEvent>,
}

/// a scheduled event where the `Leader` is expected to create a block
pub struct ScheduledEvent {
    pub leader_output: LeaderEvent,
    pub expected_time: SystemTime,
}

impl LeaderSchedule {
    /// create a new schedule based on the [`TaskParameters`] and the `Leader`
    /// settings.
    ///
    /// [`TaskParameters`]: ./struct.TaskParameters.html
    ///
    pub fn new(
        logger: Logger,
        leader_id: &LeaderId,
        enclave: &Enclave,
        task_parameters: &TaskParameters,
    ) -> Self {
        let leadership = &task_parameters.leadership;
        let era = leadership.era();
        let number_of_slots_per_epoch = era.slots_per_epoch();
        let now = std::time::SystemTime::now();

        let mut schedule = LeaderSchedule {
            events: DelayQueue::with_capacity(number_of_slots_per_epoch as usize),
        };

        let logger = logger.new(o!(
            "epoch" => leadership.epoch(),
        ));

        for slot_idx in 0..number_of_slots_per_epoch {
            schedule.schedule(
                logger.new(o!("epoch_slot" => slot_idx)),
                now,
                leader_id,
                enclave,
                task_parameters,
                slot_idx as u32,
            );
        }

        schedule
    }

    #[inline]
    fn schedule(
        &mut self,
        logger: Logger,
        now: std::time::SystemTime,
        leader_id: &LeaderId,
        enclave: &Enclave,
        task_parameters: &TaskParameters,
        slot_idx: u32,
    ) {
        let leadership = &task_parameters.leadership;
        let slot = task_parameters
            .leadership
            .era()
            .from_era_to_slot(EpochPosition {
                epoch: chain_time::Epoch(leadership.epoch()),
                slot: EpochSlotOffset(slot_idx),
            });
        let slot_system_time = task_parameters
            .time_frame
            .slot_to_systemtime(slot)
            .expect("The slot should always be in the given timeframe here");

        if now < slot_system_time {
            match enclave.leadership_evaluate1(leadership, leader_id, slot_idx) {
                None => debug!(logger, "not a leader at this time"),
                Some(leader_output) => {
                    debug!(logger, "scheduling a block leader");
                    self.events.insert(
                        ScheduledEvent {
                            expected_time: slot_system_time.clone(),
                            leader_output: leader_output,
                        },
                        slot_system_time
                            .duration_since(now)
                            .expect("expect the slot scheduled system time to be in the future"),
                    );
                }
            }
        } else {
            debug!(logger, "ignoring past events...")
        }
    }
}

impl Stream for LeaderSchedule {
    type Item = Expired<ScheduledEvent>;
    type Error = tokio::timer::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.events.poll()
    }
}
