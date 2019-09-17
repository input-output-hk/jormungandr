use crate::{
    convert::{
        decode_node_id, deserialize_bytes, deserialize_repeated_bytes, encode_node_id,
        error_from_grpc, error_into_grpc, serialize_to_bytes, FromProtobuf, IntoProtobuf,
    },
    gen, PROTOCOL_VERSION,
};

use chain_core::property;

use network_core::{
    error as core_error,
    gossip::NodeId,
    server::{BlockService, FragmentService, GossipService, Node, P2pService},
};

use futures::future::{self, FutureResult};
use futures::prelude::*;
use futures::try_ready;
use tower_grpc::{self, Code, Request, Response, Status, Streaming};

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
pub struct RequestStreamForwarding<St, F>
where
    St: Stream<Error = tower_grpc::Status>,
    F: Future,
    F::Item: Sink,
    <F::Item as Sink>::SinkItem: FromProtobuf<St::Item>,
{
    state: stream_forward::State<St, F>,
}

impl<St, F> RequestStreamForwarding<St, F>
where
    St: Stream<Error = tower_grpc::Status>,
    F: Future,
    F::Item: Sink,
    <F::Item as Sink>::SinkItem: FromProtobuf<St::Item>,
{
    fn new(stream: St, future_sink: F) -> Self {
        RequestStreamForwarding {
            state: stream_forward::State::WaitingSink(future_sink, stream),
        }
    }
}

impl<St, F> Future for RequestStreamForwarding<St, F>
where
    St: Stream<Error = tower_grpc::Status>,
    F: Future<Error = core_error::Error>,
    F::Item: Sink<SinkError = core_error::Error>,
    <F::Item as Sink>::SinkItem: FromProtobuf<St::Item>,
{
    type Item = ();
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<(), core_error::Error> {
        use stream_forward::State::*;

        loop {
            let sink = match &mut self.state {
                Forwarding(future) => {
                    let _ = try_ready!(future.poll());
                    return Ok(Async::Ready(()));
                }
                WaitingSink(future_sink, _) => try_ready!(future_sink.poll()),
                Intermediate => unreachable!(),
            };
            if let WaitingSink(_, stream) = mem::replace(&mut self.state, Intermediate) {
                // Fuse the stream to work around
                // https://github.com/rust-lang-nursery/futures-rs/pull/1864
                let stream = RequestStream::new(stream).fuse();
                self.state = Forwarding(stream.forward(sink));
            } else {
                unreachable!()
            }
        }
    }
}

mod stream_forward {
    use super::{FromProtobuf, RequestStream};
    use futures::prelude::*;
    use futures::stream::{Forward, Fuse};

    pub enum State<St, F>
    where
        St: Stream<Error = tower_grpc::Status>,
        F: Future,
        F::Item: Sink,
        <F::Item as Sink>::SinkItem: FromProtobuf<St::Item>,
    {
        WaitingSink(F, St),
        Forwarding(Forward<Fuse<RequestStream<<F::Item as Sink>::SinkItem, St>>, F::Item>),
        Intermediate,
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

    pub trait Fragment: property::Fragment + mempack::Readable + Send + 'static {}

    impl<T> Fragment for T where T: property::Fragment + mempack::Readable + Send + 'static {}

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
    <T::FragmentService as FragmentService>::Fragment: protocol_bounds::Fragment,
    <T::GossipService as GossipService>::Node: protocol_bounds::Node,
{
    type HandshakeFuture = FutureResult<Response<gen::node::HandshakeResponse>, tower_grpc::Status>;
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
    type GetFragmentsStream = ResponseStream<
        gen::node::Fragment,
        <<T as Node>::FragmentService as FragmentService>::GetFragmentsStream,
    >;
    type GetFragmentsFuture = ResponseFuture<
        Self::GetFragmentsStream,
        <<T as Node>::FragmentService as FragmentService>::GetFragmentsFuture,
    >;
    type PushHeadersFuture = ResponseFuture<
        gen::node::PushHeadersResponse,
        RequestStreamForwarding<
            Streaming<gen::node::Header>,
            <T::BlockService as BlockService>::GetPushHeadersSinkFuture,
        >,
    >;
    type UploadBlocksFuture = ResponseFuture<
        gen::node::UploadBlocksResponse,
        RequestStreamForwarding<
            Streaming<gen::node::Block>,
            <T::BlockService as BlockService>::GetUploadBlocksSinkFuture,
        >,
    >;
    type BlockSubscriptionStream = ResponseStream<
        gen::node::BlockEvent,
        <<T as Node>::BlockService as BlockService>::BlockSubscription,
    >;
    type BlockSubscriptionFuture = SubscriptionFuture<
        Self::BlockSubscriptionStream,
        <T::BlockService as P2pService>::NodeId,
        <T::BlockService as BlockService>::BlockSubscriptionFuture,
    >;
    type FragmentSubscriptionStream = ResponseStream<
        gen::node::Fragment,
        <<T as Node>::FragmentService as FragmentService>::FragmentSubscription,
    >;
    type FragmentSubscriptionFuture = SubscriptionFuture<
        Self::FragmentSubscriptionStream,
        <T::FragmentService as P2pService>::NodeId,
        <T::FragmentService as FragmentService>::FragmentSubscriptionFuture,
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

    fn handshake(&mut self, _req: Request<gen::node::HandshakeRequest>) -> Self::HandshakeFuture {
        let service = match self.inner.block_service() {
            Some(service) => service,
            None => return future::err(Status::new(Code::Unimplemented, "not implemented")),
        };
        let block0 = serialize_to_bytes(&service.block0()).unwrap();
        let res = gen::node::HandshakeResponse {
            version: PROTOCOL_VERSION,
            block0,
        };
        future::ok(Response::new(res))
    }

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

    fn get_fragments(&mut self, req: Request<gen::node::FragmentIds>) -> Self::GetFragmentsFuture {
        let service = try_get_service!(self.inner.fragment_service());
        let tx_ids = match deserialize_repeated_bytes(&req.get_ref().ids) {
            Ok(tx_ids) => tx_ids,
            Err(e) => {
                return ResponseFuture::error(error_into_grpc(e));
            }
        };
        ResponseFuture::new(service.get_fragments(&tx_ids))
    }

    fn push_headers(
        &mut self,
        req: Request<Streaming<gen::node::Header>>,
    ) -> Self::PushHeadersFuture {
        let service = try_get_service!(self.inner.block_service());
        let future_sink = service.get_push_headers_sink();
        ResponseFuture::new(RequestStreamForwarding::new(req.into_inner(), future_sink))
    }

    fn upload_blocks(
        &mut self,
        req: Request<Streaming<gen::node::Block>>,
    ) -> Self::UploadBlocksFuture {
        let service = try_get_service!(self.inner.block_service());
        let future_sink = service.get_upload_blocks_sink();
        ResponseFuture::new(RequestStreamForwarding::new(req.into_inner(), future_sink))
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

    fn fragment_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Fragment>>,
    ) -> Self::FragmentSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.fragment_service());
        let subscriber = try_decode_node_id!(&req);
        let stream = RequestStream::new(req.into_inner());
        SubscriptionFuture::new(
            service.node_id(),
            service.fragment_subscription(subscriber, stream),
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
