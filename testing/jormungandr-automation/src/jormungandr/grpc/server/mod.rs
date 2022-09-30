use crate::jormungandr::{
    grpc::{
        node::{
            node_server::{Node, NodeServer},
            BlockEvent, ClientAuthRequest, ClientAuthResponse, Gossip, HandshakeRequest,
            HandshakeResponse, PeersRequest, PeersResponse, PullBlocksRequest,
            PullBlocksToTipRequest, PullHeadersRequest, PushHeadersResponse, TipRequest,
            TipResponse, UploadBlocksResponse,
        },
        types::{Block, BlockIds, Fragment, FragmentIds, Header},
    },
    Block0ConfigurationBuilder,
};
use chain_core::{
    packer::Codec,
    property::{DeserializeFromSlice, Header as BlockHeader, Serialize},
};
use chain_impl_mockchain::{block::BlockVersion, chaintypes::ConsensusVersion, key::Hash};
use std::{
    fmt,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::info;

mod builder;
mod controller;
mod data;
mod logger;
mod verifier;

pub use builder::{start_thread, MockBuilder};
pub use controller::MockController;
pub use data::MockServerData;
pub use logger::{MethodType, MockLogger};
pub use verifier::MockVerifier;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockExitCode {
    Timeout,
    Success,
}

#[derive(Clone, Debug)]
pub enum ProtocolVersion {
    Bft = 0,
    GenesisPraos = 1,
}

impl From<ConsensusVersion> for ProtocolVersion {
    fn from(from: ConsensusVersion) -> Self {
        match from {
            ConsensusVersion::Bft => Self::Bft,
            ConsensusVersion::GenesisPraos => Self::GenesisPraos,
        }
    }
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
            block_header: self
                .data
                .read()
                .unwrap()
                .tip()
                .map_err(|e| tonic::Status::internal(format!("invalid tip {}", e)))?
                .serialize_as_vec()
                .map_err(|e| tonic::Status::internal(format!("cannot serialize header {}", e)))?,
        };
        Ok(Response::new(tip_response))
    }

    async fn peers(
        &self,
        _request: tonic::Request<PeersRequest>,
    ) -> Result<tonic::Response<PeersResponse>, tonic::Status> {
        info!(method = %MethodType::GetPeers, "Get peers request received");
        let data = self.data.read().unwrap();
        // Gossip struct serde, jormungandr/src/topology/gossip.rs
        let mut codec = chain_core::packer::Codec::new(Vec::new());
        let bytes = data.profile().gossip().as_ref();
        if bytes.len() > 512 {
            panic!("gossip size overflow");
        }
        codec.put_be_u16(bytes.len() as u16).unwrap();
        codec.put_bytes(bytes).unwrap();
        Ok(Response::new(PeersResponse {
            peers: vec![codec.into_inner()],
        }))
    }
    async fn get_blocks(
        &self,
        request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetBlocksStream>, tonic::Status> {
        info!(
            method = %MethodType::GetBlocks,
            "Get blocks request received"
        );

        let block_ids = request.into_inner();

        let mut blocks = vec![];

        for block_id in block_ids.ids.iter() {
            let block_hash =
                Hash::deserialize_from_slice(&mut Codec::new(block_id.as_slice())).unwrap();

            let mut block = self
                .data
                .read()
                .unwrap()
                .get_block(block_hash)
                .map_err(|_| tonic::Status::not_found(format!("{} not available", block_hash)));

            if self.data.read().unwrap().invalid_block0_hash()
                && block
                    .as_ref()
                    .map(|b| b.header().version() == BlockVersion::Genesis)
                    .unwrap_or(false)
            {
                block = Ok(Block0ConfigurationBuilder::new().build().to_block());
            }

            blocks.push(block);
        }

        let (tx, rx) = mpsc::channel(blocks.len());

        for block in blocks {
            tx.send(block.map(|b| {
                let mut codec = Codec::new(vec![]);
                b.serialize(&mut codec).unwrap();
                Block {
                    content: codec.into_inner(),
                }
            }))
            .await
            .unwrap();
        }

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
        request: tonic::Request<PullBlocksRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksStream>, tonic::Status> {
        info!(
            method = %MethodType::PullBlocks,
            "PullBlocks request received",
        );
        let request = request.into_inner();
        let (distance, block_iter) = {
            let data = self.data.read().unwrap();

            let (from, to) = (request.from[0].as_ref(), request.to.as_ref());

            let distance = data
                .storage()
                // This ignores all checkpoints except the first one, we don't need it for now
                .is_ancestor(from, to)
                .map_err(|e| tonic::Status::not_found(e.to_string()))?
                .ok_or_else(|| tonic::Status::invalid_argument("from is not an ancestor of to"))?;
            let iter = data.storage().iter(request.to.as_ref(), distance).unwrap();
            (distance, iter)
        };

        let (tx, rx) = mpsc::channel(distance as usize);
        for block in block_iter {
            tx.send(
                block
                    .map(|b| Block {
                        content: b.as_ref().into(),
                    })
                    .map_err(|e| tonic::Status::aborted(e.to_string())),
            )
            .await
            .unwrap();
        }
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
