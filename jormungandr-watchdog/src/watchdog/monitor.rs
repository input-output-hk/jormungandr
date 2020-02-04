use crate::watchdog::{ControlCommand, ControlHandler};
use std::future::Future;
use tokio::{
    runtime,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub struct WatchdogMonitor {
    runtime: runtime::Runtime,
    control_command: mpsc::Sender<ControlCommand>,
    watchdog_finished: oneshot::Receiver<()>,
}

impl WatchdogMonitor {
    pub(crate) fn new(
        runtime: runtime::Runtime,
        control_command: mpsc::Sender<ControlCommand>,
        watchdog_finished: oneshot::Receiver<()>,
    ) -> Self {
        WatchdogMonitor {
            runtime,
            control_command,
            watchdog_finished,
        }
    }

    pub fn control(&self) -> ControlHandler {
        ControlHandler::new(self.control_command.clone())
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    pub fn wait_finished(self) {
        let Self {
            mut runtime,
            watchdog_finished,
            ..
        } = self;

        runtime.block_on(async move { watchdog_finished.await.unwrap() })
    }
}
