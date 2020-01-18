use crate::watchdog::{ControlCommand, ControlHandler};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::sync::{mpsc, oneshot};

pub struct WatchdogMonitor {
    control_command: mpsc::Sender<ControlCommand>,
    watchdog_finished: oneshot::Receiver<()>,
}

impl WatchdogMonitor {
    pub(crate) fn new(
        control_command: mpsc::Sender<ControlCommand>,
        watchdog_finished: oneshot::Receiver<()>,
    ) -> Self {
        WatchdogMonitor {
            control_command,
            watchdog_finished,
        }
    }

    pub fn control(&self) -> ControlHandler {
        ControlHandler::new(self.control_command.clone())
    }
}

impl Future for WatchdogMonitor {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let pinned = std::pin::Pin::new(&mut self.get_mut().watchdog_finished);

        match pinned.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => Poll::Ready(()),
        }
    }
}
