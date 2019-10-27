use crate::error::Error;

use futures::Future;

// derive
use thiserror::Error;

/// Error detailing the reason of a failure to process
/// the client-streamed request.
#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("request stream error")]
    Inbound(#[source] Error),
    #[error("failed to decode stream data")]
    Decoding(#[source] Error),
    #[error("failed to process request data")]
    Sink(#[source] Error),
}

impl ProcessingError {
    /// Converts the client-streamed request error into the underlying
    /// protocol error, losing the origin information.
    #[inline]
    pub fn flatten(self) -> Error {
        use ProcessingError::*;
        match self {
            Inbound(e) | Decoding(e) | Sink(e) => e,
        }
    }
}

/// Application-defined handler of client-streamed requests.
pub trait MapResponse {
    /// Type of the response value.
    type Response;

    /// A future resulting in the response.
    type ResponseFuture: Future<Item = Self::Response, Error = Error> + Send + 'static;

    /// Observes the termination result of the request stream and returns
    /// a future used to produce the response.
    ///
    /// An implementation of this method gives the application a way to
    /// observe errors that may occur while receiving and processing
    /// items of the request stream, report the termination of the stream,
    /// and produce a response for the network peer.
    fn on_stream_termination(&mut self, res: Result<(), ProcessingError>) -> Self::ResponseFuture;
}
