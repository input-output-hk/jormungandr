use crate::watchdog::{ControlCommand, WatchdogQuery};
use std::future::Future;
use tokio::{
    runtime,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

pub struct WatchdogMonitor {
    runtime: runtime::Runtime,
    _service_runtimes: Vec<runtime::Runtime>,
    control_command: mpsc::Sender<ControlCommand>,
    watchdog_finished: oneshot::Receiver<()>,
}

impl WatchdogMonitor {
    pub(crate) fn new(
        runtime: runtime::Runtime,
        service_runtimes: Vec<runtime::Runtime>,
        control_command: mpsc::Sender<ControlCommand>,
        watchdog_finished: oneshot::Receiver<()>,
    ) -> Self {
        WatchdogMonitor {
            runtime,
            control_command,
            watchdog_finished,
            _service_runtimes: service_runtimes,
        }
    }

    pub fn control(&self) -> WatchdogQuery {
        WatchdogQuery::new(self.runtime.handle().clone(), self.control_command.clone())
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
