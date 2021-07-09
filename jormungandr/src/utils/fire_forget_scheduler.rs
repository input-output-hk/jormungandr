use futures::{
    prelude::*,
    task::{Context, Poll},
};
use std::{collections::VecDeque, pin::Pin, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc::{
    self,
    error::TrySendError,
    Receiver, Sender,
};
use tokio_util::time::delay_queue::{DelayQueue, Key};

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to send a command: {0}")]
    CommandSend(&'static str),
    #[error("command queue closed")]
    CommandQueueClosed,
    #[error("timer error")]
    Timer(#[from] tokio::time::error::Error),
}

impl<T> From<TrySendError<T>> for Error {
    fn from(error: TrySendError<T>) -> Self {
        let cause = match error {
            TrySendError::Closed(_) => "channel closed",
            TrySendError::Full(_) => "no available capacity",
        };
        Error::CommandSend(cause)
    }
}

/// Schedule for fire-forget tasks
///
/// Each task has an ID (TID) and can be launched onto a worker identified with an ID (WID).
/// Launching is defined by a closure (Launcher), which also accepts additional data (Data).
/// Multiple instances of task with same TID, but different WID may be running in parallel.
/// Launcher must be quick and non-blocking.
///
/// When task is scheduled, it's queued. When it's finally launched, a timeout is started.
/// Until it runs out or task is declared complete it's considered running and it consumes parallel
/// task execution limits. Finishing with timeout doesn't make task completed and other queued
/// instances with same TID, but different WID may be run. Timed out instances are considered
/// failed and aren't rescheduled. Finishing by declaration task completion cancels all task
/// instances with the same TID.
///
/// Scheduling tasks and declaring them complete is possible with `FireForgetScheduler`.
///
/// The scheduler is a future that never resolves, it's used only to drive itself on executor.
/// It requires a valid Tokio context.
pub struct FireForgetSchedulerFuture<TID, WID, Data, Launcher>
where
    TID: Clone + PartialEq,
    WID: Clone + PartialEq,
    Launcher: Fn(TID, WID, Data),
{
    command_sender: FireForgetScheduler<TID, WID, Data>,
    command_receiver: Receiver<Command<TID, WID, Data>>,
    scheduled: VecDeque<ScheduledTask<TID, WID, Data>>,
    running: Vec<RunningTask<TID, WID>>,
    timeouts: DelayQueue<TimedOutTask<TID, WID>>,
    launcher: Launcher,
    max_running_same_task: usize,
    timeout: Duration,
}

pub struct FireForgetSchedulerConfig {
    /// How many tasks can be run in parallel
    pub max_running: usize,
    /// How many tasks with the same TID can be run in prallel
    pub max_running_same_task: usize,
    /// Size of command channel between `FireForgetScheduler` and `FireForgetSchedulerFuture`
    pub command_channel_size: usize,
    /// Launched task timeout after which it's considered failed if not declared complete
    pub timeout: Duration,
}

impl<TID, WID, Data, Launcher> FireForgetSchedulerFuture<TID, WID, Data, Launcher>
where
    TID: Clone + PartialEq,
    WID: Clone + PartialEq,
    Launcher: Fn(TID, WID, Data),
{
    /// Launcher controls how tasks will be started. It must be quick and non-blocking.
    pub fn new(config: &FireForgetSchedulerConfig, launcher: Launcher) -> Self {
        let (sender, command_receiver) = mpsc::channel(config.command_channel_size);
        let command_sender = FireForgetScheduler { sender };
        FireForgetSchedulerFuture {
            command_sender,
            command_receiver,
            scheduled: VecDeque::new(),
            running: Vec::with_capacity(config.max_running),
            timeouts: DelayQueue::with_capacity(config.max_running),
            launcher,
            max_running_same_task: config.max_running_same_task,
            timeout: config.timeout,
        }
    }

    pub fn scheduler(&self) -> FireForgetScheduler<TID, WID, Data> {
        self.command_sender.clone()
    }

    fn schedule(&mut self, task: ScheduledTask<TID, WID, Data>) {
        let scheduled_opt = self
            .scheduled
            .iter_mut()
            .find(|scheduled| scheduled.is_scheduled(&task));
        match scheduled_opt {
            Some(scheduled) => scheduled.update_data(task),
            None => {
                self.scheduled.push_back(task);
                self.try_run_scheduled();
            }
        }
    }

    fn declare_completed(&mut self, task: TID) {
        self.scheduled
            .retain(|scheduled| !scheduled.is_completed(&task));
        let timeouts = &mut self.timeouts;
        self.running.retain(|running| {
            if running.is_completed(&task) {
                timeouts.remove(&running.timeout_key);
                false
            } else {
                true
            }
        });
        self.try_run_scheduled();
    }

    fn declare_timed_out(&mut self, timed_out: TimedOutTask<TID, WID>) {
        self.running
            .retain(|running| !running.is_timed_out(&timed_out));
        self.try_run_scheduled();
    }

    fn try_run_scheduled(&mut self) {
        while self.running.len() < self.running.capacity() {
            let scheduled = match self.pop_next_runnable_task() {
                Some(scheduled) => scheduled,
                None => break,
            };
            let timeout_key = self.timeouts.insert(scheduled.to_timed_out(), self.timeout);
            self.running.push(scheduled.to_running(timeout_key));
            scheduled.launch(&self.launcher);
        }
    }

    fn pop_next_runnable_task(&mut self) -> Option<ScheduledTask<TID, WID, Data>> {
        self.scheduled
            .iter()
            .position(|scheduled| self.task_run_count(scheduled) < self.max_running_same_task)
            .and_then(|run_idx| self.scheduled.remove(run_idx))
    }

    fn task_run_count(&self, scheduled: &ScheduledTask<TID, WID, Data>) -> usize {
        self.running
            .iter()
            .filter(|running| running.is_running_same_task(scheduled))
            .count()
    }
}

impl<TID, WID, Data, Launcher> Future for FireForgetSchedulerFuture<TID, WID, Data, Launcher>
where
    TID: Clone + PartialEq + Unpin,
    WID: Clone + PartialEq + Unpin,
    Data: Unpin,
    Launcher: Fn(TID, WID, Data) + Unpin,
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let inner = Pin::into_inner(self);
        while let Poll::Ready(command_opt) = Pin::new(&mut inner.command_receiver).poll_recv(cx) {
            match command_opt {
                None => return Poll::Ready(Err(Error::CommandQueueClosed)),
                Some(Command::Schedule { task }) => inner.schedule(task),
                Some(Command::DeclareCompleted { task }) => inner.declare_completed(task),
            }
        }
        while let Poll::Ready(Some(expired)) = Pin::new(&mut inner.timeouts).poll_expired(cx) {
            match expired {
                Ok(expired) => inner.declare_timed_out(expired.into_inner()),
                Err(err) => return Poll::Ready(Err(Error::Timer(err))),
            }
        }
        Poll::Pending
    }
}

pub struct FireForgetScheduler<TID, WID, Data> {
    sender: Sender<Command<TID, WID, Data>>,
}

impl<TID, WID, Data> Clone for FireForgetScheduler<TID, WID, Data> {
    fn clone(&self) -> Self {
        FireForgetScheduler {
            sender: self.sender.clone(),
        }
    }
}

impl<TID, WID, Data> FireForgetScheduler<TID, WID, Data> {
    /// Schedules a task to be launched.
    /// If task with same TID and WID is already queued, it has no effect.
    pub fn schedule(&mut self, tid: TID, wid: WID, data: Data) -> Result<(), Error> {
        self.try_send(Command::Schedule {
            task: ScheduledTask { tid, wid, data },
        })
    }

    /// Declares all tasks with given TID completed.
    /// Queued instances will be canceled and running ones will be considered finished.
    pub fn declare_completed(&mut self, task: TID) -> Result<(), Error> {
        self.try_send(Command::DeclareCompleted { task })
    }

    fn try_send(&mut self, command: Command<TID, WID, Data>) -> Result<(), Error> {
        self.sender.try_send(command).map_err(Into::into)
    }
}

enum Command<TID, WID, Data> {
    Schedule { task: ScheduledTask<TID, WID, Data> },
    DeclareCompleted { task: TID },
}

struct ScheduledTask<TID, WID, Data> {
    tid: TID,
    wid: WID,
    data: Data,
}

impl<TID, WID, Data> ScheduledTask<TID, WID, Data>
where
    TID: Clone + PartialEq,
    WID: Clone + PartialEq,
{
    fn to_running(&self, timeout_key: Key) -> RunningTask<TID, WID> {
        RunningTask {
            tid: self.tid.clone(),
            wid: self.wid.clone(),
            timeout_key,
        }
    }

    fn to_timed_out(&self) -> TimedOutTask<TID, WID> {
        TimedOutTask {
            tid: self.tid.clone(),
            wid: self.wid.clone(),
        }
    }

    fn is_completed(&self, task: &TID) -> bool {
        self.tid == *task
    }

    fn is_scheduled(&self, scheduled: &Self) -> bool {
        self.tid == scheduled.tid && self.wid == scheduled.wid
    }

    fn update_data(&mut self, other: Self) {
        self.data = other.data
    }

    fn launch(self, launcher: impl Fn(TID, WID, Data)) {
        launcher(self.tid, self.wid, self.data);
    }
}

struct RunningTask<TID, WID> {
    tid: TID,
    wid: WID,
    timeout_key: Key,
}

impl<TID, WID> RunningTask<TID, WID>
where
    TID: PartialEq,
    WID: PartialEq,
{
    fn is_timed_out(&self, timed_out: &TimedOutTask<TID, WID>) -> bool {
        self.tid == timed_out.tid && self.wid == timed_out.wid
    }

    fn is_completed(&self, task: &TID) -> bool {
        self.tid == *task
    }

    fn is_running_same_task<Data>(&self, scheduled: &ScheduledTask<TID, WID, Data>) -> bool {
        self.tid == scheduled.tid
    }
}

struct TimedOutTask<TID, WID> {
    tid: TID,
    wid: WID,
}
