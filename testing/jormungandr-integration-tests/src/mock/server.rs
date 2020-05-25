use slog::Drain;

use serde::{Deserialize, Serialize};

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::common::file_utils;

use bytes::Bytes;
use std::{fmt, iter};

use tonic::{transport::Server, Request, Response, Status};

pub use node::{
    node_server::{Node, NodeServer},
    {
        Block, BlockEvent, BlockIds, Fragment, FragmentIds, Gossip, HandshakeRequest,
        HandshakeResponse, Header, PeersRequest, PeersResponse, PullBlocksToTipRequest,
        PullHeadersRequest, PushHeadersResponse, TipRequest, TipResponse, UploadBlocksResponse,
    },
};

use std::str::FromStr;

use chain_impl_mockchain::{
    block::Block as LibBlock, fragment::Fragment as LibFragment, header::Header as LibHeader,
    key::Hash,
};

use futures::Stream;
use std::pin::Pin;
use tokio::sync::mpsc;

pub mod node {
    tonic::include_proto!("iohk.chain.node"); // The string specified here must match the proto package name
}

#[derive(Debug)]
pub struct MockLogger {
    pub log_file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum MethodType {
    Init,
    Handshake,
    PullBlocksToTip,
    Tip,
    GetBlocks,
    GetHeaders,
    GetFragments,
    GetPeers,
    PullHeaders,
    PushHeaders,
    UploadBlocks,
    BlockSubscription,
    FragmentSubscription,
    GossipSubscription,
}

impl fmt::Display for MethodType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub enum Level {
    WARN,
    INFO,
    ERRO,
}

#[derive(Serialize, Deserialize)]
pub struct LogEntry {
    pub msg: String,
    pub level: Level,
    pub ts: String,
    pub method: MethodType,
}

impl MockLogger {
    pub fn new(log_file_path: PathBuf) -> Self {
        MockLogger { log_file_path }
    }

    pub fn get_log_content(&self) -> String {
        file_utils::read_file(&self.log_file_path)
    }

    fn parse_line_as_entry(&self, line: &String) -> LogEntry {
        self.try_parse_line_as_entry(line).unwrap_or_else(|error| panic!(
            "Cannot parse log line into json '{}': {}. Please ensure json logger is used for node. Full log content: {}",
            &line,
            error,
            self.get_log_content()
        ))
    }

    fn try_parse_line_as_entry(&self, line: &String) -> Result<LogEntry, impl std::error::Error> {
        serde_json::from_str(&line)
    }

    pub fn get_log_entries(&self) -> impl Iterator<Item = LogEntry> + '_ {
        self.get_lines_from_log()
            .map(move |x| self.parse_line_as_entry(&x))
    }

    pub fn executed_at_least_once(&self, method: MethodType) -> bool {
        self.get_log_entries().any(|entry| entry.method == method)
    }

    fn get_lines_from_log(&self) -> impl Iterator<Item = String> {
        let file = File::open(self.log_file_path.clone())
            .expect(&format!("cannot find log file: {:?}", &self.log_file_path));
        let reader = BufReader::new(file);
        reader.lines().map(|line| line.unwrap())
    }
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
    genesis_hash: Hash,
    tip: Hash,
    protocol: ProtocolVersion,
    log: slog::Logger,
}

impl JormungandrServerImpl {
    fn init_logger(log_path: PathBuf) -> slog::Logger {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .unwrap();

        let drain = slog_json::Json::new(file).add_default_keys().build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let log = slog::Logger::root(drain, o!());
        log
    }

    pub fn new(
        port: u16,
        genesis_hash: Hash,
        tip: Hash,
        protocol: ProtocolVersion,
        log_path: PathBuf,
    ) -> Self {
        let log = JormungandrServerImpl::init_logger(log_path);
        info!(log, "{}", format!("mock node started on port {}", port); "method" => MethodType::Init.to_string());

        JormungandrServerImpl {
            genesis_hash,
            tip,
            protocol,
            log,
        }
    }
}

#[tonic::async_trait]
impl Node for JormungandrServerImpl {
    type PullBlocksToTipStream = mpsc::Receiver<Result<Block, Status>>;
    type GetBlocksStream = mpsc::Receiver<Result<Block, Status>>;
    type PullHeadersStream = mpsc::Receiver<Result<Header, Status>>;
    type GetHeadersStream = mpsc::Receiver<Result<Header, Status>>;
    type GetFragmentsStream = mpsc::Receiver<Result<Fragment, Status>>;
    type BlockSubscriptionStream =
        Pin<Box<dyn Stream<Item = Result<BlockEvent, Status>> + Send + Sync + 'static>>;
    type FragmentSubscriptionStream =
        Pin<Box<dyn Stream<Item = Result<Fragment, Status>> + Send + Sync + 'static>>;
    type GossipSubscriptionStream =
        Pin<Box<dyn Stream<Item = Result<Gossip, Status>> + Send + Sync + 'static>>;

    async fn handshake(
        &self,
        _request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        info!(self.log,"Handshake method recieved";"method" => MethodType::Handshake.to_string());

        let reply = node::HandshakeResponse {
            version: self.protocol.clone() as u32,
            block0: self.genesis_hash.as_ref().to_vec(),
        };

        Ok(Response::new(reply))
    }

    async fn tip(
        &self,
        _request: tonic::Request<TipRequest>,
    ) -> Result<tonic::Response<TipResponse>, tonic::Status> {
        info!(self.log,"Tip request recieved";"method" => MethodType::Tip.to_string());

        let tip_response = TipResponse {
            block_header: self.tip.as_ref().to_vec(),
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
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(rx))
    }
    async fn get_headers(
        &self,
        _request: tonic::Request<BlockIds>,
    ) -> Result<tonic::Response<Self::GetHeadersStream>, tonic::Status> {
        info!(self.log,"Get headers request recieved";"method" => MethodType::GetHeaders.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(rx))
    }
    async fn get_fragments(
        &self,
        _request: tonic::Request<FragmentIds>,
    ) -> Result<tonic::Response<Self::GetFragmentsStream>, tonic::Status> {
        info!(self.log,"Get fragments request recieved";"method" => MethodType::GetFragments.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(rx))
    }
    async fn pull_headers(
        &self,
        _request: tonic::Request<PullHeadersRequest>,
    ) -> Result<tonic::Response<Self::PullHeadersStream>, tonic::Status> {
        info!(self.log,"Pull Headers request recieved";"method" => MethodType::PullHeaders.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(rx))
    }
    async fn pull_blocks_to_tip(
        &self,
        _request: tonic::Request<PullBlocksToTipRequest>,
    ) -> Result<tonic::Response<Self::PullBlocksToTipStream>, tonic::Status> {
        info!(self.log,"PullBlocksToTip request recieved";"method" => MethodType::PullBlocksToTip.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(rx))
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
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(Box::pin(rx)))
    }

    async fn fragment_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Fragment>>,
    ) -> Result<tonic::Response<Self::FragmentSubscriptionStream>, tonic::Status> {
        info!(self.log,"Fragment subscription event recieved";"method" => MethodType::FragmentSubscription.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(Box::pin(rx)))
    }
    async fn gossip_subscription(
        &self,
        _request: tonic::Request<tonic::Streaming<Gossip>>,
    ) -> Result<tonic::Response<Self::GossipSubscriptionStream>, tonic::Status> {
        info!(self.log,"Gossip subscription event recieved";"method" => MethodType::GossipSubscription.to_string());
        let (mut tx, rx) = mpsc::channel(0);
        Ok(Response::new(Box::pin(rx)))
    }
}
