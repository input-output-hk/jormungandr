use crate::convert::{error_into_grpc, IntoProtobuf};
use network_core::error as core_error;

use futures::prelude::*;
use tower_grpc::Status;

use std::marker::PhantomData;

#[must_use = "streams do nothing unless polled"]
pub struct ResponseStream<T, S> {
    inner: S,
    _phantom: PhantomData<T>,
}

impl<T, S> ResponseStream<T, S>
where
    S: Stream,
    S::Item: IntoProtobuf<T>,
{
    pub fn new(stream: S) -> Self {
        ResponseStream {
            inner: stream,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Stream for ResponseStream<T, S>
where
    S: Stream<Error = core_error::Error>,
    S::Item: IntoProtobuf<T>,
{
    type Item = T;
    type Error = Status;

    fn poll(&mut self) -> Poll<Option<T>, Status> {
        poll_and_convert_stream(&mut self.inner)
    }
}

fn poll_and_convert_stream<T, S>(stream: &mut S) -> Poll<Option<T>, Status>
where
    S: Stream<Error = core_error::Error>,
    S::Item: IntoProtobuf<T>,
{
    match stream.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
        Ok(Async::Ready(Some(item))) => {
            let item = item.into_message()?;
            Ok(Async::Ready(Some(item)))
        }
        Err(e) => Err(error_into_grpc(e)),
    }
}

impl<S, T> IntoProtobuf<ResponseStream<T, S>> for S
where
    S: Stream,
    S::Item: IntoProtobuf<T>,
{
    fn into_message(self) -> Result<ResponseStream<T, S>, Status> {
        let stream = ResponseStream::new(self);
        Ok(stream)
    }
}
