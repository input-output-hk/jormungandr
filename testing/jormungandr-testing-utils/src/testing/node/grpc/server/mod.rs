pub use super::proto::{
    node_server::{Node, NodeServer},
    {
        Block, BlockEvent, BlockIds, ClientAuthRequest, ClientAuthResponse, Fragment, FragmentIds,
        Gossip, HandshakeRequest, HandshakeResponse, Header, PeersRequest, PeersResponse,
        PullBlocksRequest, PullBlocksToTipRequest, PullHeadersRequest, PushHeadersResponse,
        TipRequest, TipResponse, UploadBlocksResponse,
    },
};

use slog::Drain;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;

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
    log: slog::Logger,
}

impl JormungandrServerImpl {
    fn init_logger(log_path: impl AsRef<Path>) -> slog::Logger {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .unwrap();

        let drain = slog_json::Json::new(file).add_default_keys().build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, o!())
    }

    pub fn new(data: Arc<RwLock<MockServerData>>, log_path: impl AsRef<Path>, port: u16) -> Self {
        let log = JormungandrServerImpl::init_logger(log_path);
        info!(log, "{}", format!("mock node started on port {}",port); "method" => MethodType::Init.to_string());
        JormungandrServerImpl { data, log }
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
        info!(self.log,"Handshake method recieved";"method" => MethodType::Handshake.to_string());

        let request = request.into_inner();
        let client_nonce = &request.nonce;

        let mut data = self.data.write().await;
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
        info!(self.log, "ClientAuth request recieved"; "method" => MethodType::ClientAuth.to_string());
        let data = self.data.read().await;
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
        info!(self.log,"Tip request recieved";"method" => MethodType::Tip.to_string());
        let tip_response = TipResponse {
            block_header: self.data.read().await.tip().to_raw().to_vec(),
        };
        Ok(Response::new(tip_response))
    }

    async fn peers(
        &self,
        _request: tonic::Request<PeersRequest>,
    ) -> Result<tonic::Response<PeersResponse>, tonic::Status> {
        info!(self.log,"Get peers request recieved";"method" => MethodType::GetPeers.to_string());
        Ok(Response::new(PeersResponse::default()))
    }
    async fn get_blocks(
        &self,
        _request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetBlocksStream>, tonic::Status> {
        info!(self.log,"Get blocks request recieved";"method" => MethodType::GetBlocks.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn get_headers(
        &self,
        _request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetHeadersStream>, tonic::Status> {
        info!(self.log,"Get headers request recieved";"method" => MethodType::GetHeaders.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn get_fragments(
        &self,
        _request: tonic::Request<FragmentIds>,
    ) -> Result<tonic::Response<Self::GetFragmentsStream>, tonic::Status> {
        info!(self.log,"Get fragments request recieved";"method" => MethodType::GetFragments.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_headers(
        &self,
        _request: tonic::Request<PullHeadersRequest>,
    ) -> Result<tonic::Response<Self::PullHeadersStream>, tonic::Status> {
        info!(self.log,"Pull Headers request recieved";"method" => MethodType::PullHeaders.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_blocks(
        &self,
        _request: tonic::Request<PullBlocksRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksStream>, tonic::Status> {
        info!(self.log,"PullBlocks request recieved";"method" => MethodType::PullBlocks.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn pull_blocks_to_tip(
        &self,
        _request: tonic::Request<PullBlocksToTipRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksToTipStream>, tonic::Status> {
        info!(self.log,"PullBlocksToTip request recieved";"method" => MethodType::PullBlocksToTip.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn push_headers(
        &self,
        _request: tonic::Request<tonic::Streaming<Header>>,
    ) -> Result<tonic::Response<PushHeadersResponse>, tonic::Status> {
        info!(self.log,"Push headers method recieved";"method" => MethodType::PushHeaders.to_string());
        Ok(Response::new(PushHeadersResponse::default()))
    }
    async fn upload_blocks(
        &self,
        _request: tonic::Request<tonic::Streaming<Block>>,
    ) -> Result<tonic::Response<UploadBlocksResponse>, tonic::Status> {
        info!(self.log,"Upload blocks method recieved";"method" => MethodType::UploadBlocks.to_string());
        Ok(Response::new(UploadBlocksResponse::default()))
    }

    async fn block_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Header>>,
    ) -> Result<tonic::Response<Self::BlockSubscriptionStream>, tonic::Status> {
        info!(self.log,"Block subscription event recieved";"method" => MethodType::BlockSubscription.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn fragment_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Fragment>>,
    ) -> Result<tonic::Response<Self::FragmentSubscriptionStream>, tonic::Status> {
        info!(self.log,"Fragment subscription event recieved";"method" => MethodType::FragmentSubscription.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
    async fn gossip_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Gossip>>,
    ) -> Result<tonic::Response<Self::GossipSubscriptionStream>, tonic::Status> {
        info!(self.log,"Gossip subscription event recieved";"method" => MethodType::GossipSubscription.to_string());
        let (_tx, rx) = mpsc::channel(1);
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
