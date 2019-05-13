use crate::{
    blockcfg::{Epoch, Leader},
    blockchain::Tip,
    intercom::BlockMsg,
    leadership::{EpochParameters, Leadership, Task, TaskParameters},
    transaction::TPoolR,
    utils::{async_msg::MessageBox, task::TokioServiceInfo},
};
use std::sync::Arc;
use tokio::{
    prelude::*,
    sync::{mpsc, watch},
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

        slog_info!(service_info.logger(), "preparing");

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
        slog_info!(self.service_info.logger(), "starting");

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
                slog_crit!(error_logger, "Error in the Leadership Process" ; "reason" => error.to_string())
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
