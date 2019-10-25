use super::response_future::ResponseFuture;
use crate::convert::{encode_node_id, IntoProtobuf};
use chain_core::property;
use network_core::error as core_error;
use network_core::gossip::NodeId;

use futures::prelude::*;
use tower_grpc::{self, Code, Status};

use std::mem;

#[must_use = "futures do nothing unless polled"]
pub enum SubscriptionFuture<T, Id, F> {
    Normal {
        inner: ResponseFuture<T, F>,
        node_id: Id,
    },
    Failed(Status),
    Finished,
}

impl<T, Id, F> SubscriptionFuture<T, Id, F>
where
    Id: NodeId,
    F: Future,
    F::Item: IntoProtobuf<T>,
{
    pub fn new(node_id: Id, future: F) -> Self {
        SubscriptionFuture::Normal {
            inner: ResponseFuture::new(future),
            node_id,
        }
    }
}

impl<T, Id, F> SubscriptionFuture<T, Id, F> {
    pub fn error(status: Status) -> Self {
        SubscriptionFuture::Failed(status)
    }

    pub fn unimplemented() -> Self {
        SubscriptionFuture::Failed(Status::new(Code::Unimplemented, "not implemented"))
    }
}

impl<T, Id, F> Future for SubscriptionFuture<T, Id, F>
where
    Id: NodeId + property::Serialize,
    F: Future<Error = core_error::Error>,
    F::Item: IntoProtobuf<T>,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Status> {
        if let SubscriptionFuture::Normal { inner, node_id } = self {
            let mut res = try_ready!(inner.poll());
            encode_node_id(node_id, res.metadata_mut())?;
            Ok(Async::Ready(res))
        } else {
            match mem::replace(self, SubscriptionFuture::Finished) {
                SubscriptionFuture::Normal { .. } => unreachable!(),
                SubscriptionFuture::Failed(status) => Err(status),
                SubscriptionFuture::Finished => panic!("polled a finished subscription future"),
            }
        }
    }
}
