use crate::convert::{error_from_grpc, FromProtobuf};
use network_core::error as core_error;

use futures::prelude::*;

use std::marker::PhantomData;

#[must_use = "streams do nothing unless polled"]
pub struct RequestStream<T, S> {
    inner: S,
    _phantom: PhantomData<T>,
}

impl<T, S> RequestStream<T, S> {
    pub fn new(inner: S) -> Self {
        RequestStream {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, S> Stream for RequestStream<T, S>
where
    S: Stream<Error = tower_grpc::Status>,
    T: FromProtobuf<S::Item>,
{
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Option<T>, core_error::Error> {
        match self.inner.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(msg))) => {
                let item = T::from_message(msg)?;
                Ok(Async::Ready(Some(item)))
            }
            Err(e) => Err(error_from_grpc(e)),
        }
    }
}
