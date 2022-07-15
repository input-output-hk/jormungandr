use super::{JormungandrServerImpl, MockController, MockLogger, MockServerData, ProtocolVersion};
use crate::jormungandr::{configuration::get_available_port, grpc::server::NodeServer};
use chain_core::property::Serialize;
use chain_impl_mockchain::{block::Block, key::Hash};
use chain_storage::BlockStore;
use futures::FutureExt;
use std::{
    io::{Result, Write},
    net::SocketAddr,
    sync::{
        mpsc::{sync_channel, SyncSender},
        Arc, RwLock,
    },
};
use tokio::sync::oneshot;
use tonic::transport::Server;

pub struct MockBuilder {
    mock_port: Option<u16>,
    genesis_block: Option<Block>,
    protocol_version: ProtocolVersion,
    invalid_block0_hash: bool,
}

impl Default for MockBuilder {
    fn default() -> Self {
        Self {
            mock_port: None,
            genesis_block: None,
            protocol_version: ProtocolVersion::GenesisPraos,
            invalid_block0_hash: false,
        }
    }
}

impl MockBuilder {
    pub fn with_port(&mut self, mock_port: u16) -> &mut Self {
        self.mock_port = Some(mock_port);
        self
    }

    pub fn with_genesis_block(&mut self, block: Block) -> &mut Self {
        self.genesis_block = Some(block);
        self
    }

    pub fn with_protocol_version(&mut self, protocol_version: ProtocolVersion) -> &mut Self {
        self.protocol_version = protocol_version;
        self
    }

    pub fn with_invalid_block0_hash(&mut self, invalid_block0_hash: bool) -> &mut Self {
        self.invalid_block0_hash = invalid_block0_hash;
        self
    }

    pub fn build_data(&self) -> Arc<RwLock<MockServerData>> {
        let storage = BlockStore::memory(Hash::zero_hash().as_bytes().to_owned()).unwrap();
        let block0 = if let Some(block) = self.genesis_block.clone().take() {
            block
        } else {
            // Block contents do not really matter.
            // A full block is used just to make the storage consistent and reuse code
            super::data::block0()
        };

        let data = MockServerData::new(
            block0.header().hash(),
            self.protocol_version.clone(),
            format!(
                "127.0.0.1:{}",
                self.mock_port.unwrap_or_else(get_available_port)
            )
            .parse::<SocketAddr>()
            .unwrap(),
            storage,
            self.invalid_block0_hash,
        );

        data.put_block(&block0).unwrap();
        data.set_tip(block0.header().hash().serialize_as_vec().unwrap().as_ref())
            .unwrap();

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

pub fn start_thread(data: Arc<RwLock<MockServerData>>) -> MockController {
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
