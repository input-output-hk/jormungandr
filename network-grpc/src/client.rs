mod connect;

use crate::{
    convert::{decode_node_id, encode_node_id, error_from_grpc, serialize_to_vec},
    gen::{self, node::client as gen_client},
};

use chain_core::property;
use network_core::{
    client::{block::BlockService, gossip::GossipService, P2pService},
    error as core_error,
    gossip::{self, Gossip, NodeId},
};

use futures::future::Executor;
use tokio::prelude::*;
use tower_add_origin::{self, AddOrigin};
use tower_grpc::{BoxBody, Request, Streaming};
use tower_h2::client::Background;

use std::marker::PhantomData;

pub use connect::{Connect, ConnectError, ConnectFuture};

/// Traits setting additional bounds for blockchain entities
/// that need to be satisfied for the protocol implementation.
///
/// The traits are auto-implemented for the types that satisfy the necessary
/// bounds. These traits then can be used in lieu of the lengthy bound clauses,
/// so that, should the implementation requrements change, only these trait
/// definitions and blanket implementations need to be modified.
pub mod chain_bounds {
    use chain_core::property;

    pub trait BlockId: property::BlockId + property::Deserialize
    // Alas, bounds on associated types of the supertrait do not have
    // the desired effect:
    // https://github.com/rust-lang/rust/issues/32722
    //
    // where
    //    <Self as property::Deserialize>::Error: Send + Sync,
    {
    }

    impl<T> BlockId for T where T: property::BlockId + property::Deserialize {}

    pub trait BlockDate: property::BlockDate + property::FromStr {}

    impl<T> BlockDate for T where T: property::BlockDate + property::FromStr {}

    pub trait Header: property::Header + property::Deserialize {}

    impl<T> Header for T
    where
        T: property::Header + property::Deserialize,
        <T as property::Header>::Id: BlockId,
        <T as property::Header>::Date: BlockDate,
    {
    }

    pub trait Block: property::Block + property::HasHeader + property::Deserialize {}

    impl<T> Block for T
    where
        T: property::Block + property::HasHeader + property::Deserialize,
        <T as property::Block>::Id: BlockId,
        <T as property::Block>::Date: BlockDate,
        <T as property::HasHeader>::Header: Header,
    {
    }
}

/// A trait that fixes the types of protocol entities and the bounds
/// these entities need to satisfy for the protocol implementation.
pub trait ProtocolConfig {
    type BlockId: chain_bounds::BlockId;
    type BlockDate: chain_bounds::BlockDate;
    type Header: chain_bounds::Header + property::Header<Id = Self::BlockId, Date = Self::BlockDate>;
    type Block: chain_bounds::Block
        + property::Block<Id = Self::BlockId, Date = Self::BlockDate>
        + property::HasHeader<Header = Self::Header>;
    type Node: gossip::Node;
}

/// gRPC client for blockchain node.
///
/// This type encapsulates the gRPC protocol client that can
/// make connections and perform requests towards other blockchain nodes.
pub struct Connection<P, T, E>
where
    P: ProtocolConfig,
{
    service: gen_client::Node<AddOrigin<tower_h2::client::Connection<T, E, BoxBody>>>,
    node_id: Option<<P::Node as gossip::Node>::Id>,
}

type GrpcUnaryFuture<R> = tower_grpc::client::unary::ResponseFuture<
    R,
    tower_h2::client::ResponseFuture,
    tower_h2::RecvBody,
>;

type GrpcServerStreamingFuture<R> =
    tower_grpc::client::server_streaming::ResponseFuture<R, tower_h2::client::ResponseFuture>;

type GrpcBidiStreamingFuture<R> =
    tower_grpc::client::streaming::ResponseFuture<R, tower_h2::client::ResponseFuture>;

pub struct ResponseFuture<T, R> {
    state: unary_future::State<T, R>,
}

impl<T, R> ResponseFuture<T, R> {
    fn new(future: GrpcUnaryFuture<R>) -> Self {
        ResponseFuture {
            state: unary_future::State::Pending(future),
        }
    }
}

pub struct ResponseStreamFuture<T, R> {
    inner: GrpcServerStreamingFuture<R>,
    _phantom: PhantomData<T>,
}

impl<T, R> ResponseStreamFuture<T, R> {
    fn new(inner: GrpcServerStreamingFuture<R>) -> Self {
        ResponseStreamFuture {
            inner,
            _phantom: PhantomData,
        }
    }
}

pub struct SubscriptionFuture<T, Id, R> {
    inner: GrpcBidiStreamingFuture<R>,
    _phantom: PhantomData<(T, Id)>,
}

impl<T, Id, R> SubscriptionFuture<T, Id, R> {
    fn new(inner: GrpcBidiStreamingFuture<R>) -> Self {
        SubscriptionFuture {
            inner,
            _phantom: PhantomData,
        }
    }
}

pub struct ResponseStream<T, R> {
    inner: Streaming<R, tower_h2::RecvBody>,
    _phantom: PhantomData<T>,
}

mod unary_future {
    use super::{core_error, GrpcUnaryFuture, ResponseFuture};
    use crate::convert::{error_from_grpc, FromProtobuf};
    use futures::prelude::*;
    use std::marker::PhantomData;
    use tower_grpc::{Response, Status};

    fn poll_and_convert_response<T, R, F>(future: &mut F) -> Poll<T, core_error::Error>
    where
        T: FromProtobuf<R>,
        F: Future<Item = Response<R>, Error = Status>,
    {
        match future.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(res)) => {
                let item = T::from_message(res.into_inner())?;
                Ok(Async::Ready(item))
            }
            Err(e) => Err(error_from_grpc(e)),
        }
    }

    pub enum State<T, R> {
        Pending(GrpcUnaryFuture<R>),
        Finished(PhantomData<T>),
    }

    impl<T, R> Future for ResponseFuture<T, R>
    where
        R: prost::Message + Default,
        T: FromProtobuf<R>,
    {
        type Item = T;
        type Error = core_error::Error;

        fn poll(&mut self) -> Poll<T, core_error::Error> {
            if let State::Pending(ref mut f) = self.state {
                let res = poll_and_convert_response(f);
                if let Ok(Async::NotReady) = res {
                    return Ok(Async::NotReady);
                }
                self.state = State::Finished(PhantomData);
                res
            } else {
                match self.state {
                    State::Pending(_) => unreachable!(),
                    State::Finished(_) => panic!("polled a finished response"),
                }
            }
        }
    }
}

impl<T, R> Future for ResponseStreamFuture<T, R>
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

impl<T, Id, R> Future for SubscriptionFuture<T, Id, R>
where
    R: prost::Message + Default,
    Id: NodeId,
{
    type Item = (ResponseStream<T, R>, Id);
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, core_error::Error> {
        let res = try_ready!(self.inner.poll().map_err(error_from_grpc));
        let id = decode_node_id(res.metadata())?;
        let stream = ResponseStream {
            inner: res.into_inner(),
            _phantom: PhantomData,
        };
        Ok(Async::Ready((stream, id)))
    }
}

mod response_stream {
    use super::{core_error, ResponseStream};
    use crate::convert::{error_from_grpc, FromProtobuf};
    use futures::prelude::*;
    use tower_grpc::Status;

    fn poll_and_convert_item<T, S, R>(stream: &mut S) -> Poll<Option<T>, core_error::Error>
    where
        S: Stream<Item = R, Error = Status>,
        T: FromProtobuf<R>,
    {
        match stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(item))) => {
                let item = T::from_message(item)?;
                Ok(Async::Ready(Some(item)))
            }
            Err(e) => Err(error_from_grpc(e)),
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
            poll_and_convert_item(&mut self.inner)
        }
    }
}

pub struct RequestStream<S, R> {
    inner: S,
    _phantom: PhantomData<R>,
}

impl<S, R> RequestStream<S, R>
where
    S: Stream,
{
    fn new(inner: S) -> Self {
        RequestStream {
            inner,
            _phantom: PhantomData,
        }
    }
}

mod request_stream {
    use super::RequestStream;
    use crate::convert::IntoProtobuf;
    use futures::prelude::*;
    use tower_grpc::{Code, Status};

    fn poll_and_convert_item<S, R>(stream: &mut S) -> Poll<Option<R>, Status>
    where
        S: Stream,
        S::Item: IntoProtobuf<R>,
    {
        match stream.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::Ready(Some(item))) => {
                let item = item.into_message()?;
                Ok(Async::Ready(Some(item)))
            }
            Err(_) => Err(Status::new(Code::Unknown, "request stream failure")),
        }
    }

    impl<S, R> Stream for RequestStream<S, R>
    where
        S: Stream,
        S::Item: IntoProtobuf<R>,
    {
        type Item = R;
        type Error = Status;

        fn poll(&mut self) -> Poll<Option<R>, Status> {
            poll_and_convert_item(&mut self.inner)
        }
    }
}

impl<P, T, E> Connection<P, T, E>
where
    P: ProtocolConfig,
{
    fn new_subscription_request<R, Out>(&self, outbound: Out) -> Request<RequestStream<Out, R>>
    where
        Out: Stream + Send + 'static,
    {
        let rs = RequestStream::new(outbound);
        let mut req = Request::new(rs);
        if let Some(ref id) = self.node_id {
            encode_node_id(id, req.metadata_mut()).unwrap();
        } else {
            // In the current server-side implementation, the request
            // will be rejected as invalid without the node ID.
            // It makes the code simpler to try regardless, and there may
            // eventually be permissive node implementations.
        }
        req
    }
}

impl<P, T, E> P2pService for Connection<P, T, E>
where
    P: ProtocolConfig,
{
    type NodeId = <P::Node as gossip::Node>::Id;
}

impl<P, T, E> BlockService for Connection<P, T, E>
where
    P: ProtocolConfig,
    T: AsyncRead + AsyncWrite,
    E: Executor<Background<T, BoxBody>> + Clone,
{
    type Block = P::Block;
    type TipFuture = ResponseFuture<P::Header, gen::node::TipResponse>;

    type PullBlocksToTipStream = ResponseStream<P::Block, gen::node::Block>;
    type PullBlocksToTipFuture = ResponseStreamFuture<P::Block, gen::node::Block>;

    type GetBlocksStream = ResponseStream<P::Block, gen::node::Block>;
    type GetBlocksFuture = ResponseStreamFuture<P::Block, gen::node::Block>;

    type BlockSubscription = ResponseStream<P::Header, gen::node::Header>;
    type BlockSubscriptionFuture = SubscriptionFuture<P::Header, Self::NodeId, gen::node::Header>;

    fn tip(&mut self) -> Self::TipFuture {
        let req = gen::node::TipRequest {};
        let future = self.service.tip(Request::new(req));
        ResponseFuture::new(future)
    }

    fn pull_blocks_to_tip(&mut self, from: &[P::BlockId]) -> Self::PullBlocksToTipFuture {
        let from = serialize_to_vec(from).unwrap();
        let req = gen::node::PullBlocksToTipRequest { from };
        let future = self.service.pull_blocks_to_tip(Request::new(req));
        ResponseStreamFuture::new(future)
    }

    fn block_subscription<Out>(&mut self, outbound: Out) -> Self::BlockSubscriptionFuture
    where
        Out: Stream<Item = P::Header> + Send + 'static,
    {
        let req = self.new_subscription_request(outbound);
        let future = self.service.block_subscription(req);
        SubscriptionFuture::new(future)
    }
}

impl<P, T, E> GossipService for Connection<P, T, E>
where
    P: ProtocolConfig,
    T: AsyncRead + AsyncWrite,
    E: Executor<Background<T, BoxBody>> + Clone,
{
    type Node = P::Node;
    type GossipSubscription = ResponseStream<Gossip<P::Node>, gen::node::Gossip>;
    type GossipSubscriptionFuture =
        SubscriptionFuture<Gossip<P::Node>, Self::NodeId, gen::node::Gossip>;

    fn gossip_subscription<Out>(&mut self, outbound: Out) -> Self::GossipSubscriptionFuture
    where
        Out: Stream<Item = Gossip<P::Node>> + Send + 'static,
    {
        let req = self.new_subscription_request(outbound);
        let future = self.service.gossip_subscription(req);
        SubscriptionFuture::new(future)
    }
}
