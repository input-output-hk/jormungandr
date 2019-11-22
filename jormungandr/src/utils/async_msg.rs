//! Multiple producer, single-consumer in-memory FIFO channels with
//! asynchronous reading.

use futures::prelude::*;
use futures::sync::mpsc::{self, Receiver, Sender};
pub use futures::sync::mpsc::{SendError, TrySendError};
use slog::Logger;

/// The output end of an in-memory FIFO channel.
#[derive(Debug)]
pub struct MessageBox<Msg>(Sender<Msg>);

/// The input end of an in-memory FIFO channel.
/// This can be read asynchronously in a Tokio task using its
/// Stream implementation.
#[derive(Debug)]
pub struct MessageQueue<Msg>(Receiver<Msg>);

/// Constructs an in-memory channel and returns the output and input halves.
/// The parameter specifies the number of messages that are allowed
/// to be pending in the channel.
pub fn channel<Msg>(buffer: usize) -> (MessageBox<Msg>, MessageQueue<Msg>) {
    let (tx, rx) = mpsc::channel(buffer);
    (MessageBox(tx), MessageQueue(rx))
}

impl<Msg> MessageBox<Msg> {
    /// Sends a message over the channel.
    ///
    /// A call to this function never blocks
    /// the current thread.
    ///
    /// # Errors
    ///
    /// If the channel is full or the receiving MessageQueue has been dropped,
    /// an error is returned in `Err`.
    pub fn try_send(&mut self, a: Msg) -> Result<(), TrySendError<Msg>> {
        self.0.try_send(a)
    }

    /// Polls the channel to determine if there is guaranteed to be capacity
    /// to send at least one item without waiting.
    pub fn poll_ready(&mut self) -> Poll<(), SendError<()>> {
        self.0.poll_ready()
    }

    /// Makes a sending task from this message box instance, the message to
    /// send, and a logger instance to report errors. The returned future
    /// is suitable for spawning onto an executor.
    pub fn into_send_task(self, msg: Msg, logger: Logger) -> SendTask<Msg> {
        SendTask {
            mbox: self,
            pending: Some(msg),
            logger,
        }
    }
}

impl<Msg> Sink for MessageBox<Msg> {
    type SinkItem = Msg;
    type SinkError = SendError<Msg>;

    fn start_send(&mut self, msg: Msg) -> StartSend<Msg, SendError<Msg>> {
        self.0.start_send(msg)
    }

    fn poll_complete(&mut self) -> Poll<(), SendError<Msg>> {
        self.0.poll_complete()
    }

    fn close(&mut self) -> Poll<(), SendError<Msg>> {
        self.0.close()
    }
}

/// State for asynchronous sending of a message over a `MessageBox`
/// that can be driven as a standalone task.
pub struct SendTask<Msg> {
    mbox: MessageBox<Msg>,
    pending: Option<Msg>,
    logger: Logger,
}

impl<Msg> SendTask<Msg> {
    fn handle_mbox_error<T>(&self, err: SendError<T>) {
        error!(
            self.logger,
            "failed to enqueue message for processing";
            "reason" => %err,
        )
    }
}

impl<Msg> Future for SendTask<Msg> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        if self.pending.is_some() {
            let msg = self.pending.take().unwrap();
            let async_sink = self
                .mbox
                .start_send(msg)
                .map_err(|e| self.handle_mbox_error(e))?;
            if let AsyncSink::NotReady(msg) = async_sink {
                self.pending = Some(msg);
                return Ok(Async::NotReady);
            }
        }
        try_ready!(self
            .mbox
            .poll_complete()
            .map_err(|e| self.handle_mbox_error(e)));
        Ok(().into())
    }
}

impl<Msg> Stream for MessageQueue<Msg> {
    type Item = Msg;
    type Error = ();
    fn poll(&mut self) -> Poll<Option<Msg>, ()> {
        self.0.poll()
    }
}

impl<Msg> Clone for MessageBox<Msg> {
    fn clone(&self) -> Self {
        MessageBox(self.0.clone())
    }
}
