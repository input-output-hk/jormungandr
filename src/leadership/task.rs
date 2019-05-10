use crate::{
    blockcfg::{Epoch, Leader, LedgerParameters, LedgerStaticParameters},
    blockchain::Tip,
    leadership::{EpochParameters, Leadership},
};
use slog::Logger;
use std::sync::Arc;
use tokio::{
    prelude::*,
    sync::{mpsc, watch},
};

#[derive(Clone)]
pub struct TaskParameters {
    pub epoch: Epoch,
    pub ledger_static_parameters: LedgerStaticParameters,
    pub ledger_parameters: LedgerParameters,

    pub leadership: Arc<Leadership>,
}

pub struct Task {
    logger: Logger,
    leader: Leader,
    blockchain_tip: Tip,
    epoch_receiver: watch::Receiver<Option<TaskParameters>>,
}

impl Task {
    #[inline]
    pub fn new(
        logger: Logger,
        leader: Leader,
        blockchain_tip: Tip,
        epoch_receiver: watch::Receiver<Option<TaskParameters>>,
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
            epoch_receiver,
        }
    }

    pub fn start(mut self) -> impl Future<Item = (), Error = ()> {
        let logger = self.logger.clone();

        self.epoch_receiver
            .map_err(move |error| {
                slog_crit!(
                    logger,
                    "cannot continue with Leader task";
                    "error" => format!("{}", error)
                )
            })
            // filter_map so we don't have to do the pattern match on `Option::Nothing`.
            .filter_map(|task_parameters| task_parameters)
            .for_each(move |task_parameters| {
                // TODO
                future::ok(())
            })
    }
}
