//! Multiple producer, single-consumer in-memory FIFO channels with
//! asynchronous reading.

use futures::prelude::*;
use futures::sync::mpsc::{self, Receiver, SendError, Sender, TrySendError};

/// The output end of an in-memory FIFO channel.
pub struct MessageBox<Msg>(Sender<Msg>);

/// The input end of an in-memory FIFO channel.
/// This can be read asynchronously in a Tokio task using its
/// Stream implementation.
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
