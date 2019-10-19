//! A logging adapter for streams.

use futures::prelude::*;
use slog::Logger;

use std::fmt::{Debug, Display};

/// An extension adapter trait to augment streams with logging.
pub trait Log: Sized {
    /// Wraps the stream with a logging adapter using the given
    /// logger instance and message to record each item received
    /// from the stream. The item is formatted into a string with its `Debug`
    /// implementation and put under the `"item"` key with the message.
    fn log<M>(self, logger: Logger, message: M) -> LoggingStream<Self, M> {
        LoggingStream {
            stream: self,
            logger,
            message,
        }
    }
}

impl<S> Log for S where S: Stream {}

/// A stream adapter logging items produced by the wrapped stream.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct LoggingStream<S, M = &'static str> {
    stream: S,
    logger: Logger,
    message: M,
}

impl<S, M> Stream for LoggingStream<S, M>
where
    S: Stream,
    S::Item: Debug,
    M: Display,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match try_ready!(self.stream.poll()) {
            Some(item) => {
                debug!(self.logger, "{}", self.message; "item" => ?item);
                Ok(Some(item).into())
            }
            None => Ok(None.into()),
        }
    }
}
