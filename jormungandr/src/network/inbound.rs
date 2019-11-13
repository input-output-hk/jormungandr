use crate::intercom::{self, ReplyFuture};
use network_core::error as core_error;
use network_core::server::request_stream::{MapResponse, ProcessingError};

use slog::Logger;

fn log_stream_termination(logger: &Logger, res: &Result<(), ProcessingError>) {
    match res {
        Ok(()) => {
            debug!(logger, "request stream closed by the peer");
        }
        Err(e) => {
            debug!(
                logger,
                "request stream failed";
                "error" => ?e,
            );
        }
    }
}

// Hack: impl for a type from another module
impl<T, R> MapResponse for intercom::RequestSink<T, R, core_error::Error>
where
    R: Send + 'static,
{
    type Response = R;
    type ResponseFuture = ReplyFuture<R, core_error::Error>;

    fn on_stream_termination(&mut self, res: Result<(), ProcessingError>) -> Self::ResponseFuture {
        log_stream_termination(self.logger(), &res);
        self.take_reply_future()
    }
}
