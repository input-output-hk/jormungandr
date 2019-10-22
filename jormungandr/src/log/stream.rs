//! A logging adapter for streams.

use futures::prelude::*;
use slog::{Level, Logger, Record, RecordStatic};

use std::fmt::{self, Debug, Display};

/// An extension adapter trait to augment streams with logging.
pub trait Log: Sized {
    /// Wraps the stream with a logging adapter using the given
    /// logger instance and message to record each item received
    /// from the stream with the `Debug` level.
    ///
    /// The item is formatted into a string with its `Debug`
    /// implementation and put under the `"item"` key with the message.
    fn debug<'a, M>(self, logger: Logger, message: M) -> LoggingStream<'a, Self, M>
    where
        M: Display,
    {
        self.log_with_static(logger, message, record_static!(Level::Debug, ""))
    }

    /// Wraps the stream with a logging adapter using the given
    /// logger instance and message to record each item received
    /// from the stream with the `Trace` level.
    ///
    /// The item is formatted into a string with its `Debug`
    /// implementation and put under the `"item"` key with the message.
    fn trace<'a, M>(self, logger: Logger, message: M) -> LoggingStream<'a, Self, M>
    where
        M: Display,
    {
        self.log_with_static(logger, message, record_static!(Level::Trace, ""))
    }

    /// A helper for other trait methods and macros.
    /// This method should not be used directly.
    fn log_with_static<'a, M>(
        self,
        logger: Logger,
        message: M,
        record_static: RecordStatic<'a>,
    ) -> LoggingStream<'a, Self, M>
    where
        M: Display,
    {
        LoggingStream {
            stream: self,
            logger,
            message,
            record_static,
        }
    }
}

impl<S> Log for S where S: Stream {}

/// A stream adapter logging items produced by the wrapped stream.
#[must_use = "streams do nothing unless polled"]
pub struct LoggingStream<'a, S, M = &'a str> {
    stream: S,
    logger: Logger,
    message: M,
    record_static: RecordStatic<'a>,
}

impl<'a, S: Debug, M: Debug> Debug for LoggingStream<'a, S, M> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("LoggingStream")
            .field("stream", &self.stream)
            .field("logger", &self.logger)
            .field("message", &self.message)
            .finish()
    }
}

impl<'a, S, M> Stream for LoggingStream<'a, S, M>
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
                self.logger.log(&Record::new(
                    &self.record_static,
                    &format_args!("{}", self.message),
                    b!("item" => ?item),
                ));
                Ok(Some(item).into())
            }
            None => Ok(None.into()),
        }
    }
}
