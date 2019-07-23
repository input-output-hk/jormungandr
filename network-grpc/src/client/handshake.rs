use crate::{gen, convert, PROTOCOL_VERSION};
use chain_core::property;
use network_core::client::block::HandshakeError;

use futures::prelude::*;

use std::marker::PhantomData;

type ResponseFuture = tower_grpc::client::unary::ResponseFuture<
    gen::node::HandshakeResponse,
    tower_hyper::client::ResponseFuture<hyper::client::conn::ResponseFuture>,
    hyper::Body,
>;

pub struct HandshakeFuture<Id> {
    inner: ResponseFuture,
    _phantom: PhantomData<Id>,
}

impl<Id> HandshakeFuture<Id> {
    pub fn new(inner: ResponseFuture) -> Self {
        HandshakeFuture {
            inner,
            _phantom: PhantomData,
        }
    }
}

impl<Id> Future for HandshakeFuture<Id>
where
    Id: property::BlockId + property::Deserialize,
{
    type Item = Id;
    type Error = HandshakeError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let res = match self.inner.poll() {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(res)) => res.into_inner(),
            Err(status) => return Err(HandshakeError::Rpc(convert::error_from_grpc(status))),
        };
        if res.version != PROTOCOL_VERSION {
            return Err(HandshakeError::UnsupportedVersion(res.version.to_string().into()));
        }
        let block0_id = convert::deserialize_bytes(&res.block0)?;
        Ok(Async::Ready(block0_id))
    }
}