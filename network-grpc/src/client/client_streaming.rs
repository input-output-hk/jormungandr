use crate::convert::{error_from_grpc, error_into_grpc, IntoProtobuf};
use network_core::error as core_error;

use futures::prelude::*;
use tower_grpc::Status;

use std::marker::PhantomData;

type GrpcFuture<R> = tower_grpc::client::client_streaming::ResponseFuture<
    R,
    tower_hyper::client::ResponseFuture<hyper::client::conn::ResponseFuture>,
    tower_hyper::Body,
>;

pub struct ResponseFuture<R> {
    inner: GrpcFuture<R>,
}

impl<R> ResponseFuture<R> {
    pub(super) fn new(inner: GrpcFuture<R>) -> Self {
        ResponseFuture { inner }
    }
}

impl<R> Future for ResponseFuture<R>
where
    R: prost::Message + Default,
{
    type Item = ();
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<(), core_error::Error> {
        try_ready!(self.inner.poll().map_err(error_from_grpc));
        Ok(Async::Ready(()))
    }
}

pub(super) struct RequestStream<S, R> {
    inner: S,
    _phantom: PhantomData<R>,
}

impl<S, R> RequestStream<S, R>
where
    S: Stream,
{
    pub(super) fn new(inner: S) -> Self {
        RequestStream {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<S, R> Stream for RequestStream<S, R>
where
    S: Stream<Error = core_error::Error>,
    S::Item: IntoProtobuf<R>,
{
    type Item = R;
    type Error = Status;

    fn poll(&mut self) -> Poll<Option<R>, Status> {
        let maybe_item = try_ready!(self.inner.poll().map_err(error_into_grpc));
        let maybe_msg = maybe_item.map(|item| item.into_message()).transpose()?;
        Ok(Async::Ready(maybe_msg))
    }
}
