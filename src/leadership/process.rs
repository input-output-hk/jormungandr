use crate::{
    blockcfg::{BlockDate, Epoch, Leader},
    blockchain::Tip,
    intercom::BlockMsg,
    leadership::{EpochParameters, Leadership, Task, TaskParameters},
    transaction::TPoolR,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use chain_core::property::BlockDate as _;
use chain_time::era::{EpochPosition, EpochSlotOffset};
use slog::Logger;
use std::sync::Arc;
use tokio::{
    prelude::*,
    sync::{mpsc, watch},
    timer::Delay,
};

custom_error! { pub HandleEpochError
    Broadcast = "Cannot broadcast new epoch event to leader tasks, channels closed",
}

custom_error! { pub ProcessError
    EpochHandling { error: HandleEpochError, epoch: Epoch } = "Error while processing new epoch event (epoch: {epoch}): {error}",
    NewEpochReceiver { extra: String } = "Cannot accept anymore epoch events: {extra}",

}

pub struct Process {
    service_info: TokioServiceInfo,

    transaction_pool: TPoolR,
    blockchain_tip: Tip,

    block_message_box: MessageBox<BlockMsg>,

    epoch_broadcaster: watch::Sender<Option<TaskParameters>>,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
}

impl Process {
    /// create a new Leadership [`Process`].
    ///
    /// [`Process`]: ./struct.Process.html
    pub fn new(
        service_info: TokioServiceInfo,
        transaction_pool: TPoolR,
        blockchain_tip: Tip,
        block_message_box: MessageBox<BlockMsg>,
    ) -> Self {
        let (epoch_broadcaster, epoch_receiver) = watch::channel(None);

        info!(service_info.logger(), "preparing");

        Process {
            service_info,
            transaction_pool,
            blockchain_tip,
            block_message_box,
            epoch_broadcaster,
            epoch_receiver,
        }
    }

    /// start the `Leadership` process and the associated leader tasks
    pub fn start(
        mut self,
        leaders: Vec<Leader>,
        new_epoch_notifier: mpsc::Receiver<EpochParameters>,
    ) -> impl Future<Item = (), Error = ()> {
        let error_logger = self.service_info.logger().clone();
        info!(self.service_info.logger(), "starting");

        self.spawn_end_of_epoch_reminder();
        for leader in leaders {
            self.spawn_leader(leader);
        }

        new_epoch_notifier
            .map_err(|err| ProcessError::NewEpochReceiver {
                extra: format!("{}", err),
            })
            .for_each(move |epoch_parameters| {
                let epoch = epoch_parameters.epoch;
                if let Err(error) = self.handle_epoch(epoch_parameters) {
                    futures::future::err(ProcessError::EpochHandling {
                        error: error,
                        epoch: epoch,
                    })
                } else {
                    futures::future::ok(())
                }
            })
            .map_err(move |error| {
                crit!(error_logger, "Error in the Leadership Process" ; "reason" => error.to_string())
            })
    }

    /// spawn a new leader [`Task`] in the `Process` runtime.
    ///
    /// [`Task`]: ./struct.Task.html
    fn spawn_leader(&mut self, leader: Leader) {
        let epoch_receiver = self.epoch_receiver.clone();
        let blockchain_tip = self.blockchain_tip.clone();
        let logger = self.service_info.logger().clone();
        let transaction_pool = self.transaction_pool.clone();
        let block_message = self.block_message_box.clone();
        let task = Task::new(
            logger,
            leader,
            blockchain_tip,
            transaction_pool,
            epoch_receiver,
            block_message,
        );

        self.service_info.spawn(task.start())
    }

    fn spawn_end_of_epoch_reminder(&mut self) {
        let epoch_receiver = self.epoch_receiver.clone();
        let logger = self.service_info.logger().clone();
        let block_message = self.block_message_box.clone();
        let end_of_epoch_reminder = EndOfEpochReminder::new(epoch_receiver, logger, block_message);

        self.service_info.spawn(end_of_epoch_reminder.start())
    }

    /// handle incoming Epoch
    fn handle_epoch(&mut self, epoch_parameters: EpochParameters) -> Result<(), HandleEpochError> {
        let leadership =
            Leadership::new(epoch_parameters.epoch, &epoch_parameters.ledger_reference);

        let task_parameters = TaskParameters {
            epoch: epoch_parameters.epoch,
            ledger_static_parameters: epoch_parameters.ledger_static_parameters,
            ledger_parameters: epoch_parameters.ledger_parameters,
            leadership: Arc::new(leadership),
            time_frame: epoch_parameters.time_frame,
        };

        self.epoch_broadcaster
            .broadcast(Some(task_parameters))
            .map_err(|_| HandleEpochError::Broadcast)
    }
}

custom_error! {pub EndOfEpochReminderError
    EpochReceiver { extra: String } = "Cannot continue to receiver new epoch events: {extra}",
    DelayFailed { source: tokio::timer::Error } = "Delay to the end of Epoch failed",
}

struct EndOfEpochReminder {
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
    logger: Logger,
    block_message_box: MessageBox<BlockMsg>,
}
impl EndOfEpochReminder {
    fn new(
        epoch_receiver: watch::Receiver<Option<TaskParameters>>,
        logger: Logger,
        block_message_box: MessageBox<BlockMsg>,
    ) -> Self {
        EndOfEpochReminder {
            epoch_receiver,
            logger: slog::Logger::root(logger, o!(::log::KEY_TASK => "End Of Epoch Reminder")),
            block_message_box,
        }
    }

    fn start(self) -> impl Future<Item = (), Error = ()> {
        info!(self.logger, "starting");

        let handle_logger = self.logger.clone();
        let crit_logger = self.logger;
        let block_message = self.block_message_box;

        self.epoch_receiver
            .map_err(|error| EndOfEpochReminderError::EpochReceiver {
                extra: format!("{}", error),
            })
            // filter_map so we don't have to do the pattern match on `Option::Nothing`.
            .filter_map(|task_parameters| task_parameters)
            .for_each(move |task_parameters| {
                handle_epoch(block_message.clone(), handle_logger.clone(), task_parameters)
            })
            .map_err(move |error| {
                crit!(crit_logger, "critical error in the Leader task" ; "reason" => error.to_string())
            })
    }
}

fn handle_epoch(
    mut block_message: MessageBox<BlockMsg>,
    logger: Logger,
    task_parameters: TaskParameters,
) -> impl Future<Item = (), Error = EndOfEpochReminderError> {
    let era = task_parameters.leadership.era().clone();
    let time_frame = task_parameters.time_frame.clone();

    let last_slot_in_epoch = era.slots_per_epoch() - 1;

    let slot = era.from_era_to_slot(EpochPosition {
        epoch: chain_time::Epoch(task_parameters.epoch),
        slot: EpochSlotOffset(last_slot_in_epoch),
    });
    let slot_system_time = time_frame
        .slot_to_systemtime(slot)
        .expect("The slot should always be in the given timeframe here");

    let date = BlockDate::from_epoch_slot_id(task_parameters.epoch, last_slot_in_epoch);
    let now = std::time::SystemTime::now();
    let duration = slot_system_time
        .duration_since(now)
        .expect("time should always be in the future");

    debug!(
        logger,
        "scheduling end of epoch";
        "epoch" => date.epoch,
        "expected_at" => format!("{:?}", slot_system_time)
    );

    Delay::new(
        std::time::Instant::now()
            .checked_add(duration)
            .expect("That the duration is positive"),
    )
    .map_err(|error| EndOfEpochReminderError::DelayFailed { source: error })
    .and_then(move |()| {
        info!(logger, "End of epoch" ; "epoch" => date.epoch);
        block_message.send(BlockMsg::LeadershipExpectEndOfEpoch);
        future::ok(())
    })
}
