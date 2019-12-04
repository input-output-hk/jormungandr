use super::server_streaming::ResponseStream;
use crate::convert::{decode_node_id, error_from_grpc};
use chain_core::property;
use network_core::error as core_error;
use network_core::gossip::NodeId;

use futures::prelude::*;

use std::marker::PhantomData;

type GrpcFuture<R> = tower_grpc::client::streaming::ResponseFuture<
    R,
    tower_hyper::client::ResponseFuture<hyper::client::conn::ResponseFuture>,
>;

pub struct ResponseFuture<T, Id, R> {
    inner: GrpcFuture<R>,
    _phantom: PhantomData<(T, Id)>,
}

impl<T, Id, R> ResponseFuture<T, Id, R> {
    pub(super) fn new(inner: GrpcFuture<R>) -> Self {
        ResponseFuture {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<T, Id, R> Future for ResponseFuture<T, Id, R>
where
    R: prost::Message + Default,
    Id: NodeId + property::Deserialize,
{
    type Item = (ResponseStream<T, R>, Id);
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, core_error::Error> {
        let res = try_ready!(self.inner.poll().map_err(error_from_grpc));
        let id = decode_node_id(res.metadata())?;
        let stream = ResponseStream::new(res.into_inner());
        Ok(Async::Ready((stream, id)))
    }
}
