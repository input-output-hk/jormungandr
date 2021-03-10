use super::{JormungandrServerImpl, MockController, MockLogger, MockServerData, ProtocolVersion};
use assert_fs::fixture::PathChild;
use assert_fs::TempDir;
use chain_impl_mockchain::{block::Header, key::Hash, testing::TestGen};
use futures::FutureExt;
use std::io::{Result, Write};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tonic::transport::Server;

use crate::testing::node::grpc::server::NodeServer;

pub struct MockBuilder {
    mock_port: u16,
    genesis_hash: Hash,
    tip: Header,
    protocol_version: ProtocolVersion,
}

impl Default for MockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBuilder {
    pub fn new() -> Self {
        let genesis_hash: Hash = TestGen::hash();
        Self {
            mock_port: 9999,
            genesis_hash,
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
            self.genesis_hash,
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

struct ChannelWriter(SyncSender<Vec<u8>>);

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0
            .try_send(buf.to_vec())
            .expect("receiver hanged up or channel is full");
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

fn start_thread(data: Arc<RwLock<MockServerData>>, mock_port: u16) -> MockController {
    let temp_dir = TempDir::new().unwrap();
    let log_file = temp_dir.child("mock.log");
    println!(
        "mock will put logs into {}",
        log_file.path().to_string_lossy()
    );

    let (tx, rx) = sync_channel(100);
    let logger = MockLogger::new(rx);
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

    MockController::new(temp_dir, logger, shutdown_signal, data, mock_port)
}
