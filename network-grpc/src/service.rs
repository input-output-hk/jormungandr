use crate::{
    convert::{
        decode_node_id, deserialize_bytes, deserialize_repeated_bytes, encode_node_id,
        error_from_grpc, error_into_grpc, FromProtobuf, IntoProtobuf,
    },
    gen,
};

use chain_core::property;

use network_core::{
    error as core_error,
    gossip::NodeId,
    server::{
        block::BlockService, content::ContentService, gossip::GossipService, Node, P2pService,
    },
};

use futures::prelude::*;
use futures::try_ready;
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

#[must_use = "futures do nothing unless polled"]
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
    fn new(node_id: Id, future: F) -> Self {
        SubscriptionFuture::Normal {
            inner: ResponseFuture::new(future),
            node_id,
        }
    }
}

impl<T, Id, F> SubscriptionFuture<T, Id, F> {
    fn error(status: Status) -> Self {
        SubscriptionFuture::Failed(status)
    }

    fn unimplemented() -> Self {
        SubscriptionFuture::Failed(Status::new(Code::Unimplemented, "not implemented"))
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

#[must_use = "streams do nothing unless polled"]
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

#[must_use = "futures do nothing unless polled"]
pub struct UploadBlocksFuture<T, S>
where
    T: Node,
{
    stream: RequestStream<<T::BlockService as BlockService>::Block, S>,
    service: T,
    processing: Option<<T::BlockService as BlockService>::OnUploadedBlockFuture>,
}

impl<T, S> UploadBlocksFuture<T, S>
where
    T: Node,
{
    fn new(service: T, stream: S) -> Self {
        UploadBlocksFuture {
            stream: RequestStream::new(stream),
            service,
            processing: None,
        }
    }
}

impl<T, S> Future for UploadBlocksFuture<T, S>
where
    T: Node,
    S: Stream<Error = tower_grpc::Status>,
    <T::BlockService as BlockService>::Block: FromProtobuf<S::Item>,
{
    type Item = tower_grpc::Response<gen::node::UploadBlocksResponse>;
    type Error = tower_grpc::Status;

    fn poll(&mut self) -> Poll<Self::Item, tower_grpc::Status> {
        let service = self.service.block_service().ok_or_else(|| {
            tower_grpc::Status::new(tower_grpc::Code::Unimplemented, "not implemented")
        })?;
        loop {
            if let Some(ref mut future) = self.processing {
                try_ready!(future.poll().map_err(error_into_grpc));
                self.processing = None;
            }
            match self.stream.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Ok(Async::Ready(None)) => break,
                Ok(Async::Ready(Some(block))) => {
                    let future = service.on_uploaded_block(block);
                    self.processing = Some(future);
                }
                Err(_e) => {
                    // FIXME: add a core service method for error reporting
                    return Err(tower_grpc::Status::new(
                        tower_grpc::Code::Aborted,
                        "upload stream error",
                    ));
                }
            }
        }
        let res = gen::node::UploadBlocksResponse {};
        Ok(Async::Ready(tower_grpc::Response::new(res)))
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

macro_rules! try_get_service_sub {
    ($opt_ref:expr) => {
        match $opt_ref {
            None => return SubscriptionFuture::unimplemented(),
            Some(service) => service,
        }
    };
}

macro_rules! try_decode_node_id {
    ($req:expr) => {
        match decode_node_id($req.metadata()) {
            Ok(id) => id,
            Err(e) => return SubscriptionFuture::error(error_into_grpc(e)),
        }
    };
}

pub mod protocol_bounds {
    use chain_core::{mempack, property};
    use network_core::gossip;

    pub trait Block: property::Block + mempack::Readable + Send + 'static {}

    impl<T> Block for T where T: property::Block + mempack::Readable + Send + 'static {}

    pub trait Header: property::Header + mempack::Readable + Send + 'static {}

    impl<T> Header for T where T: property::Header + mempack::Readable + Send + 'static {}

    pub trait Message: property::Message + mempack::Readable + Send + 'static {}

    impl<T> Message for T where T: property::Message + mempack::Readable + Send + 'static {}

    pub trait Node:
        gossip::Node + property::Serialize + property::Deserialize + Send + 'static
    {
    }

    impl<T> Node for T where
        T: gossip::Node + property::Serialize + property::Deserialize + Send + 'static
    {
    }
}

impl<T> gen::node::server::Node for NodeService<T>
where
    T: Node + Clone,
    <T::BlockService as BlockService>::Block: protocol_bounds::Block,
    <T::BlockService as BlockService>::Header: protocol_bounds::Header,
    <T::ContentService as ContentService>::Message: protocol_bounds::Message,
    <T::GossipService as GossipService>::Node: protocol_bounds::Node,
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
    type PullHeadersStream = ResponseStream<
        gen::node::Header,
        <<T as Node>::BlockService as BlockService>::PullHeadersStream,
    >;
    type PullHeadersFuture = ResponseFuture<
        Self::PullHeadersStream,
        <<T as Node>::BlockService as BlockService>::PullHeadersFuture,
    >;
    type PullBlocksToTipStream = ResponseStream<
        gen::node::Block,
        <<T as Node>::BlockService as BlockService>::PullBlocksStream,
    >;
    type PullBlocksToTipFuture = ResponseFuture<
        Self::PullBlocksToTipStream,
        <<T as Node>::BlockService as BlockService>::PullBlocksToTipFuture,
    >;
    type GetMessagesStream = ResponseStream<
        gen::node::Message,
        <<T as Node>::ContentService as ContentService>::GetMessagesStream,
    >;
    type GetMessagesFuture = ResponseFuture<
        Self::GetMessagesStream,
        <<T as Node>::ContentService as ContentService>::GetMessagesFuture,
    >;
    type UploadBlocksFuture = UploadBlocksFuture<T, Streaming<gen::node::Block>>;
    type BlockSubscriptionStream = ResponseStream<
        gen::node::BlockEvent,
        <<T as Node>::BlockService as BlockService>::BlockSubscription,
    >;
    type BlockSubscriptionFuture = SubscriptionFuture<
        Self::BlockSubscriptionStream,
        <T::BlockService as P2pService>::NodeId,
        <T::BlockService as BlockService>::BlockSubscriptionFuture,
    >;
    type MessageSubscriptionStream = ResponseStream<
        gen::node::Message,
        <<T as Node>::ContentService as ContentService>::MessageSubscription,
    >;
    type MessageSubscriptionFuture = SubscriptionFuture<
        Self::MessageSubscriptionStream,
        <T::ContentService as P2pService>::NodeId,
        <T::ContentService as ContentService>::MessageSubscriptionFuture,
    >;
    type GossipSubscriptionStream = ResponseStream<
        gen::node::Gossip,
        <<T as Node>::GossipService as GossipService>::GossipSubscription,
    >;
    type GossipSubscriptionFuture = SubscriptionFuture<
        Self::GossipSubscriptionStream,
        <T::GossipService as P2pService>::NodeId,
        <T::GossipService as GossipService>::GossipSubscriptionFuture,
    >;

    fn tip(&mut self, _request: Request<gen::node::TipRequest>) -> Self::TipFuture {
        let service = try_get_service!(self.inner.block_service());
        ResponseFuture::new(service.tip())
    }

    fn get_blocks(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetBlocksFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_repeated_bytes(&req.get_ref().ids) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_blocks(&block_ids))
    }

    fn get_headers(&mut self, req: Request<gen::node::BlockIds>) -> Self::GetHeadersFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_repeated_bytes(&req.get_ref().ids) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_headers(&block_ids))
    }

    fn pull_headers(
        &mut self,
        req: Request<gen::node::PullHeadersRequest>,
    ) -> Self::PullHeadersFuture {
        let service = try_get_service!(self.inner.block_service());
        let from = match deserialize_repeated_bytes(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        let to = match deserialize_bytes(&req.get_ref().to) {
            Ok(block_id) => block_id,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.pull_headers(&from, &to))
    }

    fn pull_blocks_to_tip(
        &mut self,
        req: Request<gen::node::PullBlocksToTipRequest>,
    ) -> Self::PullBlocksToTipFuture {
        let service = try_get_service!(self.inner.block_service());
        let block_ids = match deserialize_repeated_bytes(&req.get_ref().from) {
            Ok(block_ids) => block_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.pull_blocks_to_tip(&block_ids))
    }

    fn get_messages(&mut self, req: Request<gen::node::MessageIds>) -> Self::GetMessagesFuture {
        let service = try_get_service!(self.inner.content_service());
        let tx_ids = match deserialize_repeated_bytes(&req.get_ref().ids) {
            Ok(tx_ids) => tx_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_messages(&tx_ids))
    }

    fn upload_blocks(
        &mut self,
        req: Request<Streaming<gen::node::Block>>,
    ) -> Self::UploadBlocksFuture {
        UploadBlocksFuture::new(self.inner.clone(), req.into_inner())
    }

    fn block_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Header>>,
    ) -> Self::BlockSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.block_service());
        let subscriber = try_decode_node_id!(&req);
        let stream = RequestStream::new(req.into_inner());
        SubscriptionFuture::new(
            service.node_id(),
            service.block_subscription(subscriber, stream),
        )
    }

    fn message_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Message>>,
    ) -> Self::MessageSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.content_service());
        let subscriber = try_decode_node_id!(&req);
        let stream = RequestStream::new(req.into_inner());
        SubscriptionFuture::new(
            service.node_id(),
            service.message_subscription(subscriber, stream),
        )
    }

    fn gossip_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Gossip>>,
    ) -> Self::GossipSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.gossip_service());
        let subscriber = try_decode_node_id!(&req);
        let stream = RequestStream::new(req.into_inner());
        SubscriptionFuture::new(
            service.node_id(),
            service.gossip_subscription(subscriber, stream),
        )
    }
}
