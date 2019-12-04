mod connect;
mod handshake;

pub mod client_streaming;
pub mod server_streaming;
pub mod subscription;
pub mod unary;

use client_streaming::RequestStream;

use crate::{
    convert::{encode_node_id, error_from_grpc, serialize_to_bytes, serialize_to_repeated_bytes},
    gen::{self, node::client as gen_client},
};

use chain_core::property;
use network_core::client::{BlockService, Client, FragmentService, GossipService, P2pService};
use network_core::error as core_error;
use network_core::gossip::{self, Gossip};
use network_core::subscription::BlockEvent;

use futures::prelude::*;
use tower_grpc::{BoxBody, Request};
use tower_request_modifier::{self, RequestModifier};

pub use connect::{Connect, ConnectError, ConnectFuture};
pub use handshake::HandshakeFuture;

/// Traits setting additional bounds for blockchain entities
/// that need to be satisfied for the protocol implementation.
///
/// The traits are auto-implemented for the types that satisfy the necessary
/// bounds. These traits then can be used in lieu of the lengthy bound clauses,
/// so that, should the implementation requrements change, only these trait
/// definitions and blanket implementations need to be modified.
pub mod chain_bounds {
    use chain_core::{mempack, property};

    pub trait BlockId: property::BlockId + mempack::Readable
    // Alas, bounds on associated types of the supertrait do not have
    // the desired effect:
    // https://github.com/rust-lang/rust/issues/32722
    //
    // where
    //    <Self as mempack::Readable>::Error: Send + Sync,
    {
    }

    impl<T> BlockId for T where T: property::BlockId + mempack::Readable {}

    pub trait BlockDate: property::BlockDate + property::FromStr {}

    impl<T> BlockDate for T where T: property::BlockDate + property::FromStr {}

    pub trait Header: property::Header + mempack::Readable {}

    impl<T> Header for T
    where
        T: property::Header + mempack::Readable,
        <T as property::Header>::Id: BlockId,
        <T as property::Header>::Date: BlockDate,
    {
    }

    pub trait Block: property::Block + property::HasHeader + mempack::Readable {}

    impl<T> Block for T
    where
        T: property::Block + property::HasHeader + mempack::Readable,
        <T as property::Block>::Id: BlockId,
        <T as property::Block>::Date: BlockDate,
        <T as property::HasHeader>::Header: Header,
    {
    }

    pub trait FragmentId: property::FragmentId + mempack::Readable {}

    impl<T> FragmentId for T where T: property::FragmentId + mempack::Readable {}

    pub trait Fragment: property::Fragment + mempack::Readable {}

    impl<T> Fragment for T
    where
        T: property::Fragment + mempack::Readable,
        <T as property::Fragment>::Id: FragmentId,
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
    type FragmentId: chain_bounds::FragmentId;
    type Fragment: chain_bounds::Fragment + property::Fragment<Id = Self::FragmentId>;
    type Node: gossip::Node<Id = Self::NodeId> + property::Serialize + property::Deserialize;
    type NodeId: gossip::NodeId + property::Serialize + property::Deserialize;
}

/// gRPC client for blockchain node.
///
/// This type encapsulates the gRPC protocol client that can
/// make connections and perform requests towards other blockchain nodes.
pub struct Connection<P>
where
    P: ProtocolConfig,
{
    service: gen_client::Node<RequestModifier<tower_hyper::client::Connection<BoxBody>, BoxBody>>,
    node_id: Option<<P::Node as gossip::Node>::Id>,
}

impl<P> Connection<P>
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

impl<P> Client for Connection<P>
where
    P: ProtocolConfig,
{
    fn poll_ready(&mut self) -> Poll<(), core_error::Error> {
        self.service.poll_ready().map_err(error_from_grpc)
    }
}

impl<P> P2pService for Connection<P>
where
    P: ProtocolConfig,
{
    type NodeId = <P::Node as gossip::Node>::Id;
}

impl<P> BlockService for Connection<P>
where
    P: ProtocolConfig,
{
    type Block = P::Block;

    type HandshakeFuture = HandshakeFuture<P::BlockId>;

    type TipFuture = unary::ResponseFuture<P::Header, gen::node::TipResponse>;

    type PullBlocksStream = server_streaming::ResponseStream<P::Block, gen::node::Block>;
    type PullBlocksToTipFuture = server_streaming::ResponseFuture<P::Block, gen::node::Block>;

    type PullHeadersStream = server_streaming::ResponseStream<P::Header, gen::node::Header>;
    type PullHeadersFuture = server_streaming::ResponseFuture<P::Header, gen::node::Header>;

    type GetBlocksStream = server_streaming::ResponseStream<P::Block, gen::node::Block>;
    type GetBlocksFuture = server_streaming::ResponseFuture<P::Block, gen::node::Block>;

    type BlockSubscription =
        server_streaming::ResponseStream<BlockEvent<P::Block>, gen::node::BlockEvent>;
    type BlockSubscriptionFuture =
        subscription::ResponseFuture<BlockEvent<P::Block>, Self::NodeId, gen::node::BlockEvent>;

    type PushHeadersFuture = client_streaming::ResponseFuture<gen::node::PushHeadersResponse>;

    type UploadBlocksFuture = client_streaming::ResponseFuture<gen::node::UploadBlocksResponse>;

    fn handshake(&mut self) -> Self::HandshakeFuture {
        let req = gen::node::HandshakeRequest {};
        let future = self.service.handshake(Request::new(req));
        HandshakeFuture::new(future)
    }

    fn tip(&mut self) -> Self::TipFuture {
        let req = gen::node::TipRequest {};
        let future = self.service.tip(Request::new(req));
        unary::ResponseFuture::new(future)
    }

    fn pull_blocks_to_tip(&mut self, from: &[P::BlockId]) -> Self::PullBlocksToTipFuture {
        let from = serialize_to_repeated_bytes(from).unwrap();
        let req = gen::node::PullBlocksToTipRequest { from };
        let future = self.service.pull_blocks_to_tip(Request::new(req));
        server_streaming::ResponseFuture::new(future)
    }

    fn pull_headers(&mut self, from: &[P::BlockId], to: &P::BlockId) -> Self::PullHeadersFuture {
        let from = serialize_to_repeated_bytes(from).unwrap();
        let to = serialize_to_bytes(to).unwrap();
        let req = gen::node::PullHeadersRequest { from, to };
        let future = self.service.pull_headers(Request::new(req));
        server_streaming::ResponseFuture::new(future)
    }

    fn get_blocks(&mut self, ids: &[P::BlockId]) -> Self::GetBlocksFuture {
        let ids = serialize_to_repeated_bytes(ids).unwrap();
        let req = gen::node::BlockIds { ids };
        let future = self.service.get_blocks(Request::new(req));
        server_streaming::ResponseFuture::new(future)
    }

    fn push_headers<S>(&mut self, headers: S) -> Self::PushHeadersFuture
    where
        S: Stream<Item = P::Header, Error = core_error::Error> + Send + 'static,
    {
        let stream = RequestStream::new(headers);
        let req = Request::new(stream);
        let future = self.service.push_headers(req);
        client_streaming::ResponseFuture::new(future)
    }

    fn upload_blocks<S>(&mut self, blocks: S) -> Self::UploadBlocksFuture
    where
        S: Stream<Item = P::Block, Error = core_error::Error> + Send + 'static,
    {
        let rs = RequestStream::new(blocks);
        let req = Request::new(rs);
        let future = self.service.upload_blocks(req);
        client_streaming::ResponseFuture::new(future)
    }

    fn block_subscription<Out>(&mut self, outbound: Out) -> Self::BlockSubscriptionFuture
    where
        Out: Stream<Item = P::Header, Error = core_error::Error> + Send + 'static,
    {
        let req = self.new_subscription_request(outbound);
        let future = self.service.block_subscription(req);
        subscription::ResponseFuture::new(future)
    }
}

impl<P> FragmentService for Connection<P>
where
    P: ProtocolConfig,
{
    type Fragment = P::Fragment;

    type GetFragmentsStream = server_streaming::ResponseStream<P::Fragment, gen::node::Fragment>;
    type GetFragmentsFuture = server_streaming::ResponseFuture<P::Fragment, gen::node::Fragment>;

    type FragmentSubscription = server_streaming::ResponseStream<P::Fragment, gen::node::Fragment>;
    type FragmentSubscriptionFuture =
        subscription::ResponseFuture<P::Fragment, Self::NodeId, gen::node::Fragment>;

    fn get_fragments(&mut self, ids: &[P::FragmentId]) -> Self::GetFragmentsFuture {
        let ids = serialize_to_repeated_bytes(ids).unwrap();
        let req = gen::node::FragmentIds { ids };
        let future = self.service.get_fragments(Request::new(req));
        server_streaming::ResponseFuture::new(future)
    }

    fn fragment_subscription<Out>(&mut self, outbound: Out) -> Self::FragmentSubscriptionFuture
    where
        Out: Stream<Item = P::Fragment, Error = core_error::Error> + Send + 'static,
    {
        let req = self.new_subscription_request(outbound);
        let future = self.service.fragment_subscription(req);
        subscription::ResponseFuture::new(future)
    }
}

impl<P> GossipService for Connection<P>
where
    P: ProtocolConfig,
{
    type Node = P::Node;
    type GossipSubscription = server_streaming::ResponseStream<Gossip<P::Node>, gen::node::Gossip>;
    type GossipSubscriptionFuture =
        subscription::ResponseFuture<Gossip<P::Node>, Self::NodeId, gen::node::Gossip>;

    fn gossip_subscription<Out>(&mut self, outbound: Out) -> Self::GossipSubscriptionFuture
    where
        Out: Stream<Item = Gossip<P::Node>, Error = core_error::Error> + Send + 'static,
    {
        let req = self.new_subscription_request(outbound);
        let future = self.service.gossip_subscription(req);
        subscription::ResponseFuture::new(future)
    }
}
