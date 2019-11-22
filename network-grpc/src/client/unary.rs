use crate::convert::{error_from_grpc, FromProtobuf};
use network_core::error as core_error;

use futures::prelude::*;

use std::marker::PhantomData;

type GrpcFuture<R> = tower_grpc::client::unary::ResponseFuture<
    R,
    tower_hyper::client::ResponseFuture<hyper::client::conn::ResponseFuture>,
    tower_hyper::Body,
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
    T: FromProtobuf<R>,
{
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<T, core_error::Error> {
        let res = try_ready!(self.inner.poll().map_err(error_from_grpc));
        let item = T::from_message(res.into_inner())?;
        Ok(Async::Ready(item))
    }
}
