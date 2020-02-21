use std::{
    fmt,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::SystemTime,
};
use tokio::{
    stream::Stream,
    sync::watch::{self, Receiver, Sender},
};

#[derive(Debug, Clone)]
pub struct StatusReader {
    status: Receiver<Status>,
    updater: Arc<Sender<Status>>,
}

#[derive(Debug)]
pub struct StatusUpdater {
    updater: Arc<Sender<Status>>,
}

/// these are the different status of the service
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Starting { since: SystemTime },
    Started { since: SystemTime },
    ShuttingDown { since: SystemTime },
    Shutdown { since: SystemTime },
}

impl Status {
    pub fn starting() -> Self {
        Status::Starting {
            since: SystemTime::now(),
        }
    }

    pub fn started() -> Self {
        Status::Started {
            since: SystemTime::now(),
        }
    }

    pub fn shutting_down() -> Self {
        Status::ShuttingDown {
            since: SystemTime::now(),
        }
    }

    pub fn shutdown() -> Self {
        Status::Shutdown {
            since: SystemTime::now(),
        }
    }

    pub fn is_shutdown(&self) -> bool {
        if let Status::Shutdown { .. } = self {
            true
        } else {
            false
        }
    }
}

impl StatusReader {
    /// create a new StatusReader
    #[allow(clippy::new_without_default)]
    pub fn new(initial: Status) -> Self {
        let (updater, status) = watch::channel(initial);
        let updater = Arc::new(updater);

        StatusReader { status, updater }
    }

    /// create a `StatusUpdater` from the given reader
    pub fn updater(&self) -> StatusUpdater {
        StatusUpdater {
            updater: Arc::clone(&self.updater),
        }
    }

    /// get the current `Status`
    pub fn status(&self) -> Status {
        *self.status.borrow()
    }

    /// be notified on status update
    pub async fn updated(&mut self) -> Option<Status> {
        self.status.recv().await
    }
}

impl StatusUpdater {
    pub fn update(&self, status: Status) {
        if self.updater.broadcast(status).is_err() {
            // if the receiver is gone, it means the watchdog dropped the
            // associated StatusReader and that it is not important to monitor
            // the status: we can ignore the broadcast error then
        }
    }
}

impl Stream for StatusReader {
    type Item = Status;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Status>> {
        Pin::new(&mut self.get_mut().status).poll_next(cx)
    }
}

impl Drop for StatusUpdater {
    fn drop(&mut self) {
        self.update(Status::ShuttingDown {
            since: SystemTime::now(),
        })
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (v, since) = match self {
            Status::Starting { since } => ("starting", since),
            Status::Started { since } => ("started", since),
            Status::ShuttingDown { since } => ("shutting down", since),
            Status::Shutdown { since } => ("shutdown", since),
        };

        // TODO: display time in better format
        write!(f, "{} (since {:?})", v, since)
    }
}
