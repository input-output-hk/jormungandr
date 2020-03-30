//! Multiple producer, single-consumer in-memory FIFO channels with
//! asynchronous reading.

use futures03::channel::mpsc::{self, Receiver, Sender};
use futures03::prelude::*;
use slog::Logger;

use std::pin::Pin;
use std::task::{Context, Poll};

pub use futures03::channel::mpsc::{SendError, TrySendError};

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

    /// Sends a message on the channel.
    ///
    /// This function should be only called after `poll_ready` has reported
    /// that the channel is ready to receive a message.
    pub fn start_send(&mut self, a: Msg) -> Result<(), SendError> {
        self.0.start_send(a)
    }

    /// Polls the channel to determine if there is guaranteed to be capacity
    /// to send at least one item without waiting.
    pub fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        self.0.poll_ready(cx)
    }

    /// Makes a sending task around this message box instance, the message to
    /// send, and a logger instance to report errors. The returned future
    /// is suitable for spawning onto an executor.
    async fn send_task(&mut self, msg: Msg, logger: Logger) {
        if let Err(e) = self.send(msg).await {
            error!(
                self.logger,
                "failed to enqueue message for processing";
                "reason" => %e,
            )
        }
    }
}

impl<Msg> Sink<Msg> for MessageBox<Msg> {
    type Error = SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        self.0.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, msg: Msg) -> Result<(), SendError> {
        self.0.start_send(msg)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        self.0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), SendError>> {
        self.0.poll_close(cx)
    }
}

/// State for asynchronous sending of a message over a `MessageBox`
/// that can be driven as a standalone task.
pub struct SendTask<Msg> {
    mbox: MessageBox<Msg>,
    pending: Option<Msg>,
    logger: Logger,
}

impl<Msg> Stream for MessageQueue<Msg> {
    type Item = Msg;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Msg>> {
        self.0.poll()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<Msg> Clone for MessageBox<Msg> {
    fn clone(&self) -> Self {
        MessageBox(self.0.clone())
    }
}
