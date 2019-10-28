mod request_stream;
mod response_future;
mod response_stream;
mod subscription;

use response_future::ResponseFuture;
use response_stream::ResponseStream;
use subscription::{Subscription, SubscriptionFuture};

use crate::{
    convert::{
        decode_node_id, deserialize_bytes, deserialize_repeated_bytes, error_into_grpc,
        serialize_to_bytes,
    },
    gen, PROTOCOL_VERSION,
};

use network_core::server::{BlockService, FragmentService, GossipService, Node, P2pService};

use futures::future::{self, FutureResult};
use tower_grpc::{self, Code, Request, Response, Status, Streaming};

#[derive(Clone, Debug)]
pub struct NodeService<T> {
    inner: T,
}

impl<T: Node> NodeService<T> {
    pub fn new(node: T) -> Self {
        NodeService { inner: node }
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

macro_rules! try_get_service_push {
    ($opt_ref:expr) => {
        match $opt_ref {
            None => return request_stream::Processing::unimplemented(),
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
    type PushHeadersFuture = request_stream::Processing<
        Streaming<gen::node::Header>,
        <T::BlockService as BlockService>::PushHeadersSink,
        gen::node::PushHeadersResponse,
    >;
    type UploadBlocksFuture = request_stream::Processing<
        Streaming<gen::node::Block>,
        <T::BlockService as BlockService>::UploadBlocksSink,
        gen::node::UploadBlocksResponse,
    >;
    type BlockSubscriptionStream = Subscription<
        gen::node::BlockEvent,
        Streaming<gen::node::Header>,
        <T::BlockService as BlockService>::BlockSubscription,
    >;
    type BlockSubscriptionFuture = SubscriptionFuture<
        gen::node::BlockEvent,
        Streaming<gen::node::Header>,
        <T::BlockService as P2pService>::NodeId,
        <T::BlockService as BlockService>::BlockSubscriptionFuture,
    >;
    type FragmentSubscriptionStream = Subscription<
        gen::node::Fragment,
        Streaming<gen::node::Fragment>,
        <T::FragmentService as FragmentService>::FragmentSubscription,
    >;
    type FragmentSubscriptionFuture = SubscriptionFuture<
        gen::node::Fragment,
        Streaming<gen::node::Fragment>,
        <T::FragmentService as P2pService>::NodeId,
        <T::FragmentService as FragmentService>::FragmentSubscriptionFuture,
    >;
    type GossipSubscriptionStream = Subscription<
        gen::node::Gossip,
        Streaming<gen::node::Gossip>,
        <T::GossipService as GossipService>::GossipSubscription,
    >;
    type GossipSubscriptionFuture = SubscriptionFuture<
        gen::node::Gossip,
        Streaming<gen::node::Gossip>,
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
        let service = try_get_service_push!(self.inner.block_service());
        let future_sink = service.push_headers();
        request_stream::Processing::new(req.into_inner(), future_sink)
    }

    fn upload_blocks(
        &mut self,
        req: Request<Streaming<gen::node::Block>>,
    ) -> Self::UploadBlocksFuture {
        let service = try_get_service_push!(self.inner.block_service());
        let future_sink = service.upload_blocks();
        request_stream::Processing::new(req.into_inner(), future_sink)
    }

    fn block_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Header>>,
    ) -> Self::BlockSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.block_service());
        let subscriber = try_decode_node_id!(&req);
        SubscriptionFuture::new(
            service.node_id(),
            req.into_inner(),
            service.block_subscription(subscriber),
        )
    }

    fn fragment_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Fragment>>,
    ) -> Self::FragmentSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.fragment_service());
        let subscriber = try_decode_node_id!(&req);
        let inbound = req.into_inner();
        SubscriptionFuture::new(
            service.node_id(),
            inbound,
            service.fragment_subscription(subscriber),
        )
    }

    fn gossip_subscription(
        &mut self,
        req: Request<Streaming<gen::node::Gossip>>,
    ) -> Self::GossipSubscriptionFuture {
        let service = try_get_service_sub!(self.inner.gossip_service());
        let subscriber = try_decode_node_id!(&req);
        let inbound = req.into_inner();
        SubscriptionFuture::new(
            service.node_id(),
            inbound,
            service.gossip_subscription(subscriber),
        )
    }
}
