use crate::{
    blockcfg::{BlockDate, Leader, LeaderOutput},
    leadership::TaskParameters,
};
use chain_core::property::BlockDate as _;
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
    pub leader_output: LeaderOutput,
    pub date: BlockDate,
    pub expected_time: SystemTime,
}

impl LeaderSchedule {
    /// create a new schedule based on the [`TaskParameters`] and the `Leader`
    /// settings.
    ///
    /// [`TaskParameters`]: ./struct.TaskParameters.html
    ///
    pub fn new(logger: Logger, leader: &Leader, task_parameters: &TaskParameters) -> Self {
        let leadership = &task_parameters.leadership;
        let era = leadership.era();
        let number_of_slots_per_epoch = era.slots_per_epoch();
        let now = std::time::SystemTime::now();

        let mut schedule = LeaderSchedule {
            events: DelayQueue::with_capacity(number_of_slots_per_epoch as usize),
        };

        let logger = Logger::root(
            logger,
            o!(
                "epoch" => leadership.epoch(),
            ),
        );

        for slot_idx in 0..number_of_slots_per_epoch {
            schedule.schedule(
                Logger::root(logger.clone(), o!("epoch_slot" => slot_idx)),
                now,
                leader,
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
        leader: &Leader,
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

        let date = BlockDate::from_epoch_slot_id(leadership.epoch(), slot_idx);

        if now < slot_system_time {
            match task_parameters.leadership.is_leader_for_date(leader, date) {
                Ok(LeaderOutput::None) => debug!(logger, "not a leader at this time"),
                Ok(leader_output) => {
                    info!(logger, "scheduling a block leader");
                    self.events.insert(
                        ScheduledEvent {
                            expected_time: slot_system_time.clone(),
                            leader_output: leader_output,
                            date: date,
                        },
                        slot_system_time
                            .duration_since(now)
                            .expect("expect the slot scheduled system time to be in the future"),
                    );
                }
                Err(error) => {
                    error!(logger, "cannot compute schedule" ; "reason" => format!("{error}", error = error))
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
