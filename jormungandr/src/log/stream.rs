// //! A logging adapter for streams.
//
// use futures::{
//     prelude::*,
//     task::{Context, Poll},
// };
// use pin_project::pin_project;
// use slog::{Level, Logger, Record, RecordStatic};
//
// use std::{
//     fmt::{self, Debug, Display},
//     pin::Pin,
// };
//
// /// An extension adapter trait to augment streams with logging.
// pub trait Log: Sized {
//     /// Wraps the stream with a logging adapter using the given
//     /// logger instance and message to record each item received
//     /// from the stream with the `Debug` level.
//     ///
//     /// The item is formatted into a string with its `Debug`
//     /// implementation and put under the `"item"` key with the message.
//     fn debug<'a, M>(self, logger: Logger, message: M) -> LoggingStream<'a, Self, M>
//     where
//         M: Display,
//     {
//         self.log_with_static(logger, message, record_static!(Level::Debug, ""))
//     }
//
//     /// Wraps the stream with a logging adapter using the given
//     /// logger instance and message to record each item received
//     /// from the stream with the `Trace` level.
//     ///
//     /// The item is formatted into a string with its `Debug`
//     /// implementation and put under the `"item"` key with the message.
//     fn trace<'a, M>(self, logger: Logger, message: M) -> LoggingStream<'a, Self, M>
//     where
//         M: Display,
//     {
//         self.log_with_static(logger, message, record_static!(Level::Trace, ""))
//     }
//
//     /// A helper for other trait methods and macros.
//     /// This method should not be used directly.
//     fn log_with_static<M>(
//         self,
//         logger: Logger,
//         message: M,
//         record_static: RecordStatic<'_>,
//     ) -> LoggingStream<'_, Self, M>
//     where
//         M: Display,
//     {
//         LoggingStream {
//             stream: self,
//             logger,
//             message,
//             record_static,
//         }
//     }
// }
//
// impl<S> Log for S where S: Stream {}
//
// /// A stream adapter logging items produced by the wrapped stream.
// #[must_use = "streams do nothing unless polled"]
// #[pin_project]
// pub struct LoggingStream<'a, S, M = &'a str> {
//     #[pin]
//     stream: S,
//     logger: Logger,
//     message: M,
//     record_static: RecordStatic<'a>,
// }
//
// impl<'a, S: Debug, M: Debug> Debug for LoggingStream<'a, S, M> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_struct("LoggingStream")
//             .field("stream", &self.stream)
//             .field("logger", &self.logger)
//             .field("message", &self.message)
//             .finish()
//     }
// }
//
// impl<'a, S, M> Stream for LoggingStream<'a, S, M>
// where
//     S: Stream,
//     S::Item: Debug,
//     M: Display,
// {
//     type Item = S::Item;
//
//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         let inner = self.project();
//         match futures::ready!(inner.stream.poll_next(cx)) {
//             Some(item) => {
//                 inner.logger.log(&Record::new(
//                     &inner.record_static,
//                     &format_args!("{}", inner.message),
//                     b!("item" => ?item),
//                 ));
//                 Poll::Ready(Some(item))
//             }
//             None => Poll::Ready(None),
//         }
//     }
// }
