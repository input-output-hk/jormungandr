use crate::{
    blockcfg::{
        BlockBuilder, BlockDate, ChainLength, Epoch, HeaderHash, Leader, LeaderOutput,
        LedgerParameters, LedgerStaticParameters,
    },
    blockchain::Tip,
    intercom::BlockMsg,
    leadership::Leadership,
    transaction::TPoolR,
    utils::async_msg::MessageBox,
};
use chain_core::property::{BlockDate as _, ChainLength as _};
use chain_time::{
    era::{EpochPosition, EpochSlotOffset},
    timeframe::TimeFrame,
};
use slog::Logger;
use std::{sync::Arc, time::SystemTime};
use tokio::{prelude::*, sync::watch, timer::DelayQueue};

custom_error! {pub HandleLeadershipError
    Schedule { source: tokio::timer::Error } = "Error in the leadership schedule",
}

custom_error! {pub TaskError
    LeadershipReceiver { extra: String } = "Cannot continue the leader task: {extra}",
    LeadershipHandle { source: HandleLeadershipError } = "Error while handling an epoch's leader schedule",
}

#[derive(Clone)]
pub struct TaskParameters {
    pub epoch: Epoch,
    pub ledger_static_parameters: LedgerStaticParameters,
    pub ledger_parameters: LedgerParameters,

    pub leadership: Arc<Leadership>,
    pub time_frame: TimeFrame,
}

pub struct ScheduledEvent {
    leader_output: LeaderOutput,
    date: BlockDate,
    expected_time: SystemTime,
    // TODO...
}

pub struct Task {
    logger: Logger,
    leader: Leader,
    blockchain_tip: Tip,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
    transaction_pool: TPoolR,
    block_message: MessageBox<BlockMsg>,
}

/// structure to prepare the schedule of a leader
pub struct LeaderSchedule {
    events: DelayQueue<ScheduledEvent>,
}

impl LeaderSchedule {
    fn new(logger: Logger, leader: &Leader, task_parameters: &TaskParameters) -> Self {
        // TODO: use parameter's number of slot per epoch
        let number_of_slots_per_epoch = 100;
        let now = std::time::SystemTime::now();

        let mut schedule = LeaderSchedule {
            events: DelayQueue::with_capacity(number_of_slots_per_epoch),
        };

        let logger = Logger::root(
            logger,
            o!(
                "epoch" => task_parameters.epoch,
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
        let slot = task_parameters
            .leadership
            .era()
            .from_era_to_slot(EpochPosition {
                epoch: chain_time::Epoch(task_parameters.epoch),
                slot: EpochSlotOffset(slot_idx),
            });
        let slot_system_time = task_parameters
            .time_frame
            .slot_to_systemtime(slot)
            .expect("The slot should always be in the given timeframe here");

        let date = BlockDate::from_epoch_slot_id(task_parameters.epoch, slot_idx);

        if now < slot_system_time {
            match task_parameters.leadership.is_leader_for_date(leader, date) {
                Ok(LeaderOutput::None) => slog_debug!(logger, "not a leader at this time"),
                Ok(leader_output) => {
                    slog_info!(logger, "scheduling a block leader");
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
                    slog_error!(logger, "cannot compute schedule" ; "reason" => format!("{error}", error = error))
                }
            }
        } else {
            slog_debug!(logger, "ignoring past events...")
        }
    }
}

impl Task {
    #[inline]
    pub fn new(
        logger: Logger,
        leader: Leader,
        blockchain_tip: Tip,
        transaction_pool: TPoolR,
        epoch_receiver: watch::Receiver<Option<TaskParameters>>,
        block_message: MessageBox<BlockMsg>,
    ) -> Self {
        let logger = Logger::root(
            logger,
            o!(
                // TODO: add some general context information here (leader alias?)
            ),
        );

        Task {
            logger,
            leader,
            blockchain_tip,
            transaction_pool,
            epoch_receiver,
            block_message,
        }
    }

    pub fn start(self) -> impl Future<Item = (), Error = ()> {
        let handle_logger = self.logger.clone();
        let crit_logger = self.logger;
        let mut leader = self.leader;
        let blockchain_tip = self.blockchain_tip;
        let transaction_pool = self.transaction_pool;
        let block_message = self.block_message;

        self.epoch_receiver
            .map_err(|error| TaskError::LeadershipReceiver {
                extra: format!("{}", error),
            })
            // filter_map so we don't have to do the pattern match on `Option::Nothing`.
            .filter_map(|task_parameters| task_parameters)
            .for_each(move |task_parameters| {
                handle_leadership(
                    block_message.clone(),
                    &mut leader,
                    handle_logger.clone(),
                    blockchain_tip.clone(),
                    transaction_pool.clone(),
                    task_parameters,
                )
                .map_err(|error| {
                    TaskError::LeadershipHandle { source: error }
                })
            })
            .map_err(move |error| {
                slog_crit!(crit_logger, "critical error in the Leader task" ; "reason" => error.to_string())
            })
    }
}

/// function that will run for the length of the Epoch associated
/// to the given leadership
///
fn handle_leadership(
    mut block_message: MessageBox<BlockMsg>,
    leader: &mut Leader,
    logger: Logger,
    blockchain_tip: Tip,
    transaction_pool: TPoolR,
    task_parameters: TaskParameters,
) -> impl Future<Item = (), Error = HandleLeadershipError> {
    let era = task_parameters.leadership.era().clone();
    let time_frame = task_parameters.time_frame.clone();

    let current_slot = time_frame.slot_at(&std::time::SystemTime::now()).expect(
        "assume we cannot only get one valid timeline and that the slot duration does not change",
    );
    let epoch_position = era
        .from_slot_to_era(current_slot)
        .expect("assume the current time is already in the era");

    // TODO: need to handle:
    //
    // * if too early for the leadership, we need to wait
    // * if too late for this leadership, log it and return
    assert!(epoch_position.epoch.0 == task_parameters.epoch);

    let schedule = LeaderSchedule::new(logger.clone(), leader, &task_parameters);

    let bft_leader = if let Some(ref leader) = &leader.bft_leader {
        leader.sig_key.clone()
    } else {
        unimplemented!()
    };

    schedule
        .map_err(|err| HandleLeadershipError::Schedule { source: err })
        .for_each(move |scheduled_event| {
            let scheduled_event = scheduled_event.into_inner();

            slog_info!(logger, "Leader scheduled event" ;
                "scheduled at_time" => format!("{:?}", scheduled_event.expected_time),
                "scheduled_at_date" => format!("{}", scheduled_event.date),
            );

            let block = prepare_block(
                &transaction_pool,
                scheduled_event.date,
                blockchain_tip.chain_length().unwrap().next(),
                blockchain_tip.hash().unwrap(),
            );

            let block = match scheduled_event.leader_output {
                LeaderOutput::None => unreachable!(),
                LeaderOutput::Bft(_) => block.make_bft_block(&bft_leader),
                LeaderOutput::GenesisPraos(_) => unimplemented!(),
            };

            block_message.send(BlockMsg::LeadershipBlock(block));

            future::ok(())
        })
}

fn prepare_block(
    transaction_pool: &TPoolR,
    date: BlockDate,
    chain_length: ChainLength,
    parent_id: HeaderHash,
) -> BlockBuilder {
    let mut bb = BlockBuilder::new();

    bb.date(date).parent(parent_id).chain_length(chain_length);
    let messages = transaction_pool.write().unwrap().collect(250 /* TODO!! */);
    bb.messages(messages);

    bb
}

impl Stream for LeaderSchedule {
    type Item = tokio::timer::delay_queue::Expired<ScheduledEvent>;
    type Error = tokio::timer::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.events.poll()
    }
}
