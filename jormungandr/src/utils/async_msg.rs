//! Multiple producer, single-consumer in-memory FIFO channels with
//! asynchronous reading.

use futures03::prelude::{Stream, Future, Sink};
use futures03::task::Poll;
use futures03::channel::mpsc::{self, Receiver, Sender};
pub use futures03::channel::mpsc::{SendError, TrySendError};
use slog::Logger;
use tonic::codegen::{Context, Pin};

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

    // /// Polls the channel to determine if there is guaranteed to be capacity
    // /// to send at least one item without waiting.
    // pub fn poll_ready(&mut self) -> Poll<Result<(), SendError>> {
    //     self.0.poll_ready()
    // }

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

impl<Msg> Sink<Msg> for MessageBox<Msg> {
    type Error = SendError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // underlying channel should be ready if it is not full
        self.0.poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, msg: Msg) -> Result<Msg, Self::Error> {
        self.0.start_send(msg)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
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

impl<Msg> SendTask<Msg> {
    fn handle_mbox_error(&self, err: SendError) {
        error!(
            self.logger,
            "failed to enqueue message for processing";
            "reason" => %err,
        )
    }
}

impl<Msg> Future for SendTask<Msg> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.pending.is_some() {
            let msg = self.pending.take().unwrap();
            let async_sink = self
                .mbox
                .start_send(msg)
                .map_err(|e| self.handle_mbox_error(e))?;
            if let Ok(msg) = async_sink {
                self.pending = Some(msg);
                return Poll::Pending;
            }
        }
        try_ready!(self
            .mbox
            .poll_complete()
            .map_err(|e| self.handle_mbox_error(e)));
        Poll::Ready(())
    }
}

impl<Msg> Stream for MessageQueue<Msg> {
    type Item = Msg;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_next()
    }
}

impl<Msg> Clone for MessageBox<Msg> {
    fn clone(&self) -> Self {
        MessageBox(self.0.clone())
    }
}
