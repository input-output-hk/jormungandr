use super::{JormungandrServerImpl, MockController, MockLogger, MockServerData, ProtocolVersion};
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::{block::Header, key::Hash, testing::TestGen};
use futures::FutureExt;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tonic::transport::Server;

pub mod node {
    tonic::include_proto!("iohk.chain.node"); // The string specified here must match the proto package name
}
use crate::mock::server::NodeServer;

pub struct MockBuilder {
    mock_port: u16,
    genesis_hash: Hash,
    tip: Header,
    protocol_version: ProtocolVersion,
}

impl MockBuilder {
    pub fn new() -> Self {
        let genesis_hash: Hash = TestGen::hash().into();
        Self {
            mock_port: 9999,
            genesis_hash: genesis_hash.clone(),
            tip: super::data::header(30, &genesis_hash),
            protocol_version: ProtocolVersion::GenesisPraos,
        }
    }

    pub fn with_port(&mut self, mock_port: u16) -> &mut Self {
        self.mock_port = mock_port;
        self
    }

    pub fn with_genesis_hash(&mut self, hash: Hash) -> &mut Self {
        self.genesis_hash = hash;
        self
    }

    pub fn with_tip(&mut self, tip: Header) -> &mut Self {
        self.tip = tip;
        self
    }

    pub fn with_protocol_version(&mut self, protocol_version: ProtocolVersion) -> &mut Self {
        self.protocol_version = protocol_version;
        self
    }

    fn build_data(&self) -> Arc<RwLock<MockServerData>> {
        let data = MockServerData::new(
            self.genesis_hash.clone(),
            self.tip.clone(),
            self.protocol_version.clone(),
        );
        Arc::new(RwLock::new(data))
    }

    pub fn build(&self) -> MockController {
        let data = self.build_data();
        start_thread(data, self.mock_port)
    }
}

fn start_thread(data: Arc<RwLock<MockServerData>>, mock_port: u16) -> MockController {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.child("mock.log");
    println!(
        "mock will put logs into {}",
        log_file.path().to_string_lossy()
    );

    let logger = MockLogger::new(log_file.path());
    let (shutdown_signal, rx) = oneshot::channel::<()>();
    let data_clone = data.clone();

    tokio::spawn(async move {
        let addr = format!("127.0.0.1:{}", mock_port);
        let mock = JormungandrServerImpl::new(data_clone, log_file.path(), mock_port);

        Server::builder()
            .add_service(NodeServer::new(mock))
            .serve_with_shutdown(addr.parse().unwrap(), rx.map(drop))
            .await
            .unwrap();
    });
    MockController::new(temp_dir, logger, shutdown_signal, data.clone(), mock_port)
}
