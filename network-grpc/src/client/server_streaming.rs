use crate::convert::{error_from_grpc, FromProtobuf};
use network_core::error as core_error;

use futures::prelude::*;
use tower_grpc::Streaming;

use std::marker::PhantomData;

type GrpcFuture<R> = tower_grpc::client::server_streaming::ResponseFuture<
    R,
    tower_hyper::client::ResponseFuture<hyper::client::conn::ResponseFuture>,
>;

pub struct ResponseFuture<T, R> {
    inner: GrpcFuture<R>,
    _phantom: PhantomData<T>,
}

impl<T, R> ResponseFuture<T, R> {
    pub(super) fn new(inner: GrpcFuture<R>) -> Self {
        ResponseFuture {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, R> Future for ResponseFuture<T, R>
where
    R: prost::Message + Default,
{
    type Item = ResponseStream<T, R>;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<ResponseStream<T, R>, core_error::Error> {
        let res = try_ready!(self.inner.poll().map_err(error_from_grpc));
        let stream = ResponseStream {
            inner: res.into_inner(),
            _phantom: PhantomData,
        };
        Ok(Async::Ready(stream))
    }
}

pub struct ResponseStream<T, R> {
    inner: Streaming<R, tower_hyper::Body>,
    _phantom: PhantomData<T>,
}

impl<T, R> ResponseStream<T, R> {
    pub(super) fn new(inner: Streaming<R, tower_hyper::Body>) -> Self {
        ResponseStream {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, R> Stream for ResponseStream<T, R>
where
    R: prost::Message + Default,
    T: FromProtobuf<R>,
{
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Option<T>, core_error::Error> {
        let maybe_msg = try_ready!(self.inner.poll().map_err(error_from_grpc));
        let maybe_item = maybe_msg.map(|msg| T::from_message(msg)).transpose()?;
        Ok(Async::Ready(maybe_item))
    }
}
