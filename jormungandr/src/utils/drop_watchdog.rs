/// Tools for multithreaded monitoring if value was dropped
/// If DropTripwire is dropped, every child DropWatchdog will be notified
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

pub struct DropTripwire {
    #[allow(dead_code)]
    sync: Receiver<()>,
    watchdog: DropWatchdog,
}

#[derive(Clone)]
pub struct DropWatchdog {
    sync: SyncSender<()>,
}

impl DropTripwire {
    pub fn new() -> Self {
        let (watchdog_sync, tripwire_sync) = sync_channel(0);
        let watchdog = DropWatchdog {
            sync: watchdog_sync,
        };
        DropTripwire {
            sync: tripwire_sync,
            watchdog,
        }
    }

    pub fn watchdog(&self) -> DropWatchdog {
        self.watchdog.clone()
    }
}

impl DropWatchdog {
    pub fn wait(&self) {
        // Sync channel is not buffered, so sending blocks until receiver
        // accepts message (which never happens) or is dropped (which is watched)
        let _ = self.sync.send(());
    }
}
