pub use super::proto::{
    node_server::{Node, NodeServer},
    {
        Block, BlockEvent, BlockIds, ClientAuthRequest, ClientAuthResponse, Fragment, FragmentIds,
        Gossip, HandshakeRequest, HandshakeResponse, Header, PeersRequest, PeersResponse,
        PullBlocksRequest, PullBlocksToTipRequest, PullHeadersRequest, PushHeadersResponse,
        TipRequest, TipResponse, UploadBlocksResponse,
    },
};

use std::sync::RwLock;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use std::fmt;
use std::sync::Arc;
use tracing::info;

mod builder;
mod controller;
mod data;
mod logger;
mod verifier;

pub use builder::MockBuilder;
pub use controller::MockController;
pub use data::{header, MockServerData};
pub use logger::{MethodType, MockLogger};
pub use verifier::MockVerifier;

#[derive(Clone, Debug, PartialEq)]
pub enum MockExitCode {
    Timeout,
    Success,
}

#[derive(Clone, Debug)]
pub enum ProtocolVersion {
    Bft = 0,
    GenesisPraos = 1,
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct JormungandrServerImpl {
    data: Arc<RwLock<MockServerData>>,
}

impl JormungandrServerImpl {
    pub fn new(data: Arc<RwLock<MockServerData>>) -> Self {
        info!(
            method = %MethodType::Init.to_string(),
            "mock node started on {}", data.read().unwrap().profile().address()
        );
        JormungandrServerImpl { data }
    }
}

#[tonic::async_trait]
impl Node for JormungandrServerImpl {
    type PullBlocksStream = ReceiverStream<Result<Block, Status>>;
    type PullBlocksToTipStream = ReceiverStream<Result<Block, Status>>;
    type GetBlocksStream = ReceiverStream<Result<Block, Status>>;
    type PullHeadersStream = ReceiverStream<Result<Header, Status>>;
    type GetHeadersStream = ReceiverStream<Result<Header, Status>>;
    type GetFragmentsStream = ReceiverStream<Result<Fragment, Status>>;
    type BlockSubscriptionStream = ReceiverStream<Result<BlockEvent, Status>>;
    type FragmentSubscriptionStream = ReceiverStream<Result<Fragment, Status>>;
    type GossipSubscriptionStream = ReceiverStream<Result<Gossip, Status>>;

    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        info!(method = %MethodType::Handshake, "Handshake method received",);

        let request = request.into_inner();
        let client_nonce = &request.nonce;

        let mut data = self.data.write().unwrap();
        let signature = data.node_signature(client_nonce);
        let nonce = data.generate_auth_nonce().to_vec();

        let reply = HandshakeResponse {
            version: data.protocol().clone() as u32,
            block0: data.genesis_hash().as_ref().to_vec(),
            node_id: data.node_id().to_vec(),
            signature,
            nonce,
        };
        Ok(Response::new(reply))
    }

    async fn client_auth(
        &self,
        request: tonic::Request<ClientAuthRequest>,
    ) -> Result<tonic::Response<ClientAuthResponse>, tonic::Status> {
        let request = request.into_inner();
        info!(
            method = %MethodType::ClientAuth,
            "ClientAuth request received",
        );
        let data = self.data.read().unwrap();
        if !data.validate_peer_node_id(&request.node_id, &request.signature) {
            return Err(Status::invalid_argument("invalid node ID or signature"));
        }
        let response = ClientAuthResponse {};
        Ok(Response::new(response))
    }

    async fn tip(
        &self,
        _request: tonic::Request<TipRequest>,
    ) -> Result<tonic::Response<TipResponse>, tonic::Status> {
        info!(method = %MethodType::Tip, "Tip request received");
        let tip_response = TipResponse {
            block_header: self.data.read().unwrap().tip().to_raw().to_vec(),
        };
        Ok(Response::new(tip_response))
    }

    async fn peers(
        &self,
        _request: tonic::Request<PeersRequest>,
    ) -> Result<tonic::Response<PeersResponse>, tonic::Status> {
        info!(method = %MethodType::GetPeers, "Get peers request received");
        use bincode::Options;
        let data = self.data.read().unwrap();
        let mut self_gossip = Vec::new();
        let config = bincode::options();
        config.with_limit(512);
        config
            .serialize_into(&mut self_gossip, data.profile().gossip().as_ref())
            .unwrap();
        Ok(Response::new(PeersResponse {
            peers: vec![self_gossip],
        }))
    }
    async fn get_blocks(
        &self,
        _request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetBlocksStream>, tonic::Status> {
        info!(
            method = %MethodType::GetBlocks,
            "Get blocks request received"
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn get_headers(
        &self,
        _request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetHeadersStream>, tonic::Status> {
        info!(
            method = %MethodType::GetHeaders,
            "Get headers request received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn get_fragments(
        &self,
        _request: tonic::Request<FragmentIds>,
    ) -> Result<tonic::Response<Self::GetFragmentsStream>, tonic::Status> {
        info!(
            method = %MethodType::GetFragments,
            "Get fragments request received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_headers(
        &self,
        _request: tonic::Request<PullHeadersRequest>,
    ) -> Result<tonic::Response<Self::PullHeadersStream>, tonic::Status> {
        info!(
            method = %MethodType::PullHeaders,
            "Pull Headers request received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_blocks(
        &self,
        _request: tonic::Request<PullBlocksRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksStream>, tonic::Status> {
        info!(
            method = %MethodType::PullBlocks,
            "PullBlocks request received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_blocks_to_tip(
        &self,
        _request: tonic::Request<PullBlocksToTipRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksToTipStream>, tonic::Status> {
        info!(
            method = %MethodType::PullBlocksToTip,
            "PullBlocksToTip request received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn push_headers(
        &self,
        _request: tonic::Request<tonic::Streaming<Header>>,
    ) -> Result<tonic::Response<PushHeadersResponse>, tonic::Status> {
        info!(
            method = %MethodType::PushHeaders,
            "Push headers method received",
        );
        Ok(Response::new(PushHeadersResponse::default()))
    }
    async fn upload_blocks(
        &self,
        _request: tonic::Request<tonic::Streaming<Block>>,
    ) -> Result<tonic::Response<UploadBlocksResponse>, tonic::Status> {
        info!(
            method = %MethodType::UploadBlocks,
            "Upload blocks method received",
        );
        Ok(Response::new(UploadBlocksResponse::default()))
    }

    async fn block_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Header>>,
    ) -> Result<tonic::Response<Self::BlockSubscriptionStream>, tonic::Status> {
        info!(
            method = %MethodType::BlockSubscription,
            "Block subscription event received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn fragment_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Fragment>>,
    ) -> Result<tonic::Response<Self::FragmentSubscriptionStream>, tonic::Status> {
        info!(
            method = %MethodType::FragmentSubscription,
            "Fragment subscription event received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn gossip_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Gossip>>,
    ) -> Result<tonic::Response<Self::GossipSubscriptionStream>, tonic::Status> {
        info!(
            method = %MethodType::GossipSubscription,
            "Gossip subscription event received",
        );
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
