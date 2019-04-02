use crate::{
    convert::{deserialize_vec, error_from_grpc, error_into_grpc, FromProtobuf, IntoProtobuf},
    gen,
};

use network_core::{
    error as core_error,
    server::{block::BlockService, content::ContentService, gossip::GossipService, Node},
};

use futures::prelude::*;
use tower_grpc::{self, Code, Request, Status, Streaming};

use std::{marker::PhantomData, mem};

#[derive(Clone, Debug)]
pub struct NodeService<T> {
    inner: T,
}

impl<T: Node> NodeService<T> {
    pub fn new(node: T) -> Self {
        NodeService { inner: node }
    }
}

pub enum ResponseFuture<T, F> {
    Pending(F),
    Failed(Status),
    Finished(PhantomData<T>),
}

impl<T, F> ResponseFuture<T, F>
where
    F: Future,
    F::Item: IntoProtobuf<T>,
{
    fn new(future: F) -> Self {
        ResponseFuture::Pending(future)
    }
}

impl<T, F> ResponseFuture<T, F> {
    fn error(status: Status) -> Self {
        ResponseFuture::Failed(status)
    }

    fn unimplemented() -> Self {
        ResponseFuture::Failed(Status::new(Code::Unimplemented, "not implemented"))
    }
}

fn poll_and_convert_response<T, F>(
    future: &mut F,
) -> Poll<tower_grpc::Response<T>, tower_grpc::Status>
where
    F: Future<Error = core_error::Error>,
    F::Item: IntoProtobuf<T>,
{
    match future.poll() {
        Ok(Async::NotReady) => Ok(Async::NotReady),
        Ok(Async::Ready(res)) => {
            let item = res.into_message()?;
            let response = tower_grpc::Response::new(item);
            Ok(Async::Ready(response))
        }
        Err(e) => Err(error_into_grpc(e)),
    }
}

fn poll_and_convert_stream<T, S>(stream: &mut S) -> Poll<Option<T>, tower_grpc::Status>
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

impl<T, F> Future for ResponseFuture<T, F>
where
    F: Future<Error = core_error::Error>,
    F::Item: IntoProtobuf<T>,
{
    type Item = tower_grpc::Response<T>;
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Status> {
        if let ResponseFuture::Pending(f) = self {
            let res = poll_and_convert_response(f);
            if let Ok(Async::NotReady) = res {
                return Ok(Async::NotReady);
            }
            *self = ResponseFuture::Finished(PhantomData);
            res
        } else {
            match mem::replace(self, ResponseFuture::Finished(PhantomData)) {
                ResponseFuture::Pending(_) => unreachable!(),
                ResponseFuture::Failed(status) => Err(status),
                ResponseFuture::Finished(_) => panic!("polled a finished response"),
            }
        }
    }
}

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
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Option<T>, tower_grpc::Status> {
        poll_and_convert_stream(&mut self.inner)
    }
}

impl<S, T> IntoProtobuf<ResponseStream<T, S>> for S
where
    S: Stream,
    S::Item: IntoProtobuf<T>,
{
    fn into_message(self) -> Result<ResponseStream<T, S>, tower_grpc::Status> {
        let stream = ResponseStream::new(self);
        Ok(stream)
    }
}

pub struct RequestStream<T, S> {
    inner: S,
    _phantom: PhantomData<T>,
}

impl<T, S> RequestStream<T, S> {
    fn new(inner: S) -> Self {
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

macro_rules! try_get_service {
    ($opt_ref:expr) => {
        match $opt_ref {
            None => return ResponseFuture::unimplemented(),
            Some(service) => service,
        }
    };
}

impl<T> gen::node::server::Node for NodeService<T>
where
    T: Node + Clone,
    <T::BlockService as BlockService>::Header: Send + 'static,
    <T::ContentService as ContentService>::Message: Send + 'static,
    <T::GossipService as GossipService>::Node: Send + 'static,
    <T::GossipService as GossipService>::NodeId: Send + 'static,
{
    type TipFuture = ResponseFuture<
        gen::node::TipResponse,
        <<T as Node>::BlockService as BlockService>::TipFuture,
    >;
    type GetBlocksStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::GetBlocksStream,
    >;
    type GetBlocksFuture = ResponseFuture<
        Self::GetBlocksStream,
        <<T as Node>::BlockService as BlockService>::GetBlocksFuture,
    >;
    type GetHeadersStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::BlockService as BlockService>::GetHeadersStream,
    >;
    type GetHeadersFuture = ResponseFuture<
        Self::GetHeadersStream,
        <<T as Node>::BlockService as BlockService>::GetHeadersFuture,
    >;
    type PullBlocksToTipStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::PullBlocksStream,
    >;
    type PullBlocksToTipFuture = ResponseFuture<
        Self::PullBlocksToTipStream,
        <<T as Node>::BlockService as BlockService>::PullBlocksFuture,
    >;
    type GetMessagesStream = ResponseStream<
        gen::node::Message,
        <<T as Node>::ContentService as ContentService>::GetMessagesStream,
    >;
    type GetMessagesFuture = ResponseFuture<
        Self::GetMessagesStream,
        <<T as Node>::ContentService as ContentService>::GetMessagesFuture,
    >;
    type BlockSubscriptionStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::BlockService as BlockService>::BlockSubscription,
    >;
    type BlockSubscriptionFuture = ResponseFuture<
        Self::BlockSubscriptionStream,
        <<T as Node>::BlockService as BlockService>::BlockSubscriptionFuture,
    >;
    type MessageSubscriptionStream = ResponseStream<
        gen::node::Message,
        <<T as Node>::ContentService as ContentService>::MessageSubscription,
    >;
    type MessageSubscriptionFuture = ResponseFuture<
        Self::MessageSubscriptionStream,
        <<T as Node>::ContentService as ContentService>::MessageSubscriptionFuture,
    >;
    type GossipSubscriptionStream = ResponseStream<
        gen::node::Gossip,
        <<T as Node>::GossipService as GossipService>::GossipSubscription,
    >;
    type GossipSubscriptionFuture = ResponseFuture<
        Self::GossipSubscriptionStream,
        <<T as Node>::GossipService as GossipService>::GossipSubscriptionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        let service = try_get_service!(self.inner.block_service());
        ResponseFuture::new(service.tip())
    }

    fn get_blocks(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetBlocksFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_blocks(&block_ids))
    }

    fn get_headers(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetHeadersFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_headers(&block_ids))
    }

    fn pull_blocks_to_tip(
        &mut self,
        req: Request<gen::node::PullBlocksToTipRequest>,
    ) -> Self::PullBlocksToTipFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_vec(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.pull_blocks_to_tip(&block_ids))
    }

    fn get_messages(&mut self, req: Request<gen::node::MessageIds>) -> Self::GetMessagesFuture {
        let service = try_get_service!(self.inner.content_service());
        let tx_ids = match deserialize_vec(&req.get_ref().id) {
            Ok(tx_ids) => tx_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_messages(&tx_ids))
    }

    fn block_subscription(
        &mut self,
        request: Request<Streaming<gen::node::Header>>,
    ) -> Self::BlockSubscriptionFuture {
        let service = try_get_service!(self.inner.block_service());
        let stream = RequestStream::new(request.into_inner());
        ResponseFuture::new(service.block_subscription(stream))
    }

    fn message_subscription(
        &mut self,
        request: Request<Streaming<gen::node::Message>>,
    ) -> Self::MessageSubscriptionFuture {
        let service = try_get_service!(self.inner.content_service());
        let stream = RequestStream::new(request.into_inner());
        ResponseFuture::new(service.message_subscription(stream))
    }

    fn gossip_subscription(
        &mut self,
        request: Request<Streaming<gen::node::Gossip>>,
    ) -> Self::GossipSubscriptionFuture {
        let service = try_get_service!(self.inner.gossip_service());
        let stream = RequestStream::new(request.into_inner());
        ResponseFuture::new(service.gossip_subscription(stream))
    }
}
