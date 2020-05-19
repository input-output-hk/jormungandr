use slog::Drain;

use serde::{Deserialize, Serialize};

use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use crate::{
    common::file_utils,
    mock::proto::{node::*, node_grpc::*},
};
use chain_impl_mockchain::key::Hash;
use grpc::{Metadata, Server};
use std::fmt;

pub fn start(
    port: u16,
    genesis_hash: Hash,
    tip: Hash,
    version: ProtocolVersion,
    log_path: PathBuf,
) -> Server {
    let mut server = grpc::ServerBuilder::new_plain();
    server.http.set_port(port);
    server.add_service(NodeServer::new_service_def(JormungandrServerImpl::new(
        port,
        genesis_hash,
        tip,
        version,
        log_path,
    )));

    let server = server.build().expect("server");
    server
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
    PullHeaders,
    PushHeaders,
    UploadBlocks,
    BlockSubscription,
    ContentSubscription,
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

impl Node for JormungandrServerImpl {
    fn handshake(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<HandshakeRequest>,
        resp: ::grpc::ServerResponseUnarySink<HandshakeResponse>,
    ) -> grpc::Result<()> {
        info!(self.log,"Handshake method recieved";"method" => MethodType::Handshake.to_string());
        let mut handshake = HandshakeResponse::new();

        handshake.set_version(self.protocol.clone() as u32);
        handshake.set_block0(self.genesis_hash.as_ref().to_vec());

        resp.finish(handshake)
    }

    fn tip(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<TipRequest>,
        resp: ::grpc::ServerResponseUnarySink<TipResponse>,
    ) -> grpc::Result<()> {
        info!(self.log,"Tip request recieved";"method" => MethodType::Tip.to_string());
        let mut tip_response = TipResponse::new();
        tip_response.set_block_header(self.tip.as_ref().to_vec());
        resp.finish(tip_response)
    }

    fn peers(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<PeersRequest>,
        resp: ::grpc::ServerResponseUnarySink<PeersResponse>,
    ) -> ::grpc::Result<()> {
        let peers_response = PeersResponse::new();
        resp.finish(peers_response)
    }

    fn get_blocks(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<BlockIds>,
        mut resp: ::grpc::ServerResponseSink<Block>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Get blocks request recieved";"method" => MethodType::GetBlocks.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn get_headers(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<BlockIds>,
        mut resp: ::grpc::ServerResponseSink<Header>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Get headers request recieved";"method" => MethodType::GetHeaders.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn get_fragments(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<FragmentIds>,
        mut resp: ::grpc::ServerResponseSink<Fragment>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Get fragments request recieved";"method" => MethodType::GetFragments.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn pull_headers(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<PullHeadersRequest>,
        mut resp: ::grpc::ServerResponseSink<Header>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Pull Headers request recieved";"method" => MethodType::PullHeaders.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn pull_blocks_to_tip(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequestSingle<PullBlocksToTipRequest>,
        mut resp: ::grpc::ServerResponseSink<Block>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"PullBlocksToTip request recieved";"method" => MethodType::PullBlocksToTip.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn push_headers(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequest<Header>,
        resp: ::grpc::ServerResponseUnarySink<PushHeadersResponse>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Push headers method recieved";"method" => MethodType::PushHeaders.to_string());
        let header_response = PushHeadersResponse::new();
        resp.finish(header_response)
    }

    fn upload_blocks(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequest<Block>,
        resp: ::grpc::ServerResponseUnarySink<UploadBlocksResponse>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Upload blocks method recieved";"method" => MethodType::UploadBlocks.to_string());
        let block_response = UploadBlocksResponse::new();
        resp.finish(block_response)
    }

    fn block_subscription(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequest<Header>,
        mut resp: ::grpc::ServerResponseSink<BlockEvent>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Block subscription event recieved";"method" => MethodType::BlockSubscription.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn fragment_subscription(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequest<Fragment>,
        mut resp: ::grpc::ServerResponseSink<Fragment>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Content subscription event recieved";"method" => MethodType::ContentSubscription.to_string());
        resp.send_trailers(Metadata::default())
    }

    fn gossip_subscription(
        &self,
        _o: ::grpc::ServerHandlerContext,
        _req: ::grpc::ServerRequest<Gossip>,
        mut resp: ::grpc::ServerResponseSink<Gossip>,
    ) -> ::grpc::Result<()> {
        info!(self.log,"Gossip subscription event recieved";"method" => MethodType::GossipSubscription.to_string());
        resp.send_trailers(Metadata::default())
    }
}
