use super::{JormungandrServerImpl, MockController, MockLogger, MockServerData, ProtocolVersion};
use chain_impl_mockchain::{block::Header, key::Hash, testing::TestGen};
use futures::FutureExt;
use std::io::{Result, Write};
use std::net::SocketAddr;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::oneshot;
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
            format!("127.0.0.1:{}", self.mock_port)
                .parse::<SocketAddr>()
                .unwrap(),
        );
        Arc::new(RwLock::new(data))
    }

    pub fn build(&self) -> MockController {
        let data = self.build_data();
        start_thread(data)
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

fn start_thread(data: Arc<RwLock<MockServerData>>) -> MockController {
    let (tx, rx) = sync_channel(100);
    let logger = MockLogger::new(rx);
    let (shutdown_signal, rx) = oneshot::channel::<()>();
    let data_clone = data.clone();
    let addr = data.read().unwrap().profile().address();

    std::thread::spawn(move || {
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_writer(move || ChannelWriter(tx.clone()))
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async move {
                let mock = JormungandrServerImpl::new(data_clone);
                Server::builder()
                    .add_service(NodeServer::new(mock))
                    .serve_with_shutdown(addr, rx.map(drop))
                    .await
                    .unwrap();
            })
        });
    });

    MockController::new(logger, shutdown_signal, data, addr.port())
}
