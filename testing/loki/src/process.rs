use ::multiaddr::{Multiaddr, Protocol};
use chain_impl_mockchain::{
    block::{Block, BlockDate},
    fee::LinearFee,
    header::Header,
};
use jormungandr_automation::{
    jormungandr::{
        grpc::{
            client::MockClientError,
            server::{
                start_thread, MockBuilder, MockController, MockServerData as NodeData,
                ProtocolVersion,
            },
            JormungandrClient,
        },
        NodeAlias, TestingDirectory,
    },
    testing::{panic, SyncNode},
};
use jormungandr_lib::{
    crypto::hash::Hash,
    interfaces::{Block0Configuration, TrustedPeer},
};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, RwLock},
};
use thor::{FragmentSender, FragmentSenderSetup};

/// An adversary-controlled node, which can deviate in every way
/// from the blockchain protocol.
///
/// It uses the same node-to-node gRPC interface as Jormungandr (although not complete)
/// so that it's able to communicate with other nodes, but the rest
/// interface and the general way of controlling its behavior might be different.
///
/// In the future, we may be interested in this node being interchangeable with
/// JormungandrProcess in terms of functionalities, but we start from what is
/// currently needed.
pub struct AdversaryNode {
    temp_dir: Option<TestingDirectory>,
    alias: String,
    node_data: Arc<RwLock<NodeData>>,
    server: Option<MockController>,
    open_client_connections: HashMap<SocketAddr, JormungandrClient>,
}

impl AdversaryNode {
    pub(crate) fn new(
        temp_dir: Option<TestingDirectory>,
        alias: String,
        node_data: Arc<RwLock<NodeData>>,
        server: Option<MockController>,
    ) -> Self {
        AdversaryNode {
            temp_dir,
            alias,
            server,
            node_data,
            open_client_connections: HashMap::new(),
        }
    }

    pub fn fragment_sender<'a, S: SyncNode + Send>(
        &self,
        setup: FragmentSenderSetup<'a, S>,
    ) -> FragmentSender<'a, S> {
        FragmentSender::new(
            self.genesis_block_hash(),
            self.fees(),
            BlockDate::first().next_epoch().into(),
            setup,
        )
    }

    pub fn alias(&self) -> NodeAlias {
        self.alias.to_string()
    }

    pub fn address(&self) -> SocketAddr {
        self.node_data.read().unwrap().profile().address()
    }

    pub fn fees(&self) -> LinearFee {
        self.block0_configuration()
            .blockchain_configuration
            .linear_fees
    }

    pub fn genesis_block_hash(&self) -> Hash {
        (*self.node_data.read().unwrap().genesis_hash()).into()
    }

    pub fn block0_configuration(&self) -> Block0Configuration {
        Block0Configuration::from_block(&self.node_data.read().unwrap().genesis_block()).unwrap()
    }

    fn p2p_public_addr(&self) -> SocketAddr {
        self.node_data.read().unwrap().profile().address()
    }

    pub fn to_trusted_peer(&self) -> TrustedPeer {
        let mut address = Multiaddr::empty();
        match self.address().ip() {
            IpAddr::V4(ip) => address.push(Protocol::Ip4(ip)),
            IpAddr::V6(ip) => address.push(Protocol::Ip6(ip)),
        }
        address.push(Protocol::Tcp(self.p2p_public_addr().port()));
        TrustedPeer { address, id: None }
    }

    pub fn steal_temp_dir(&mut self) -> Option<TestingDirectory> {
        self.temp_dir.take()
    }

    pub fn send_block_to_peer(
        &mut self,
        peer: SocketAddr,
        block: Block,
    ) -> Result<(), MockClientError> {
        let client = self
            .open_client_connections
            .entry(peer)
            .or_insert_with(|| JormungandrClient::new(peer));
        client.upload_blocks(block)
    }

    pub fn send_header_to_peer(
        &mut self,
        peer: SocketAddr,
        header: Header,
    ) -> Result<(), MockClientError> {
        let client = self
            .open_client_connections
            .entry(peer)
            .or_insert_with(|| JormungandrClient::new(peer));
        client.push_headers(header)
    }

    pub fn builder(genesis_block: Block) -> AdversaryNodeBuilder {
        AdversaryNodeBuilder::new(genesis_block)
    }

    pub fn node_data(&self) -> Arc<RwLock<NodeData>> {
        self.node_data.clone()
    }
}

impl Drop for AdversaryNode {
    fn drop(&mut self) {
        if let Some(controller) = self.server.take() {
            controller.stop();
        }

        panic::persist_dir_on_panic::<String, String>(self.temp_dir.take(), Vec::new())
    }
}

pub struct AdversaryNodeBuilder {
    alias: String,
    temp_dir: Option<TestingDirectory>,
    server_enabled: bool,
    protocol_version: ProtocolVersion,
    genesis_block: Block,
    invalid_block0_hash: bool,
}

impl AdversaryNodeBuilder {
    /// As a limitation of the current implementation, the adversary node is not able to
    /// bootstrap from peers and as such always need the full block0 to be able to function properly
    pub fn new(genesis_block: Block) -> Self {
        Self {
            alias: String::new(),
            temp_dir: None,
            server_enabled: false,
            protocol_version: ProtocolVersion::Bft,
            genesis_block,
            invalid_block0_hash: false,
        }
    }

    pub fn with_alias(self, alias: String) -> Self {
        Self { alias, ..self }
    }

    pub fn with_temp_dir(self, temp_dir: TestingDirectory) -> Self {
        Self {
            temp_dir: Some(temp_dir),
            ..self
        }
    }

    pub fn with_server_enabled(self) -> Self {
        Self {
            server_enabled: true,
            ..self
        }
    }

    pub fn with_protocol_version(self, protocol_version: ProtocolVersion) -> Self {
        Self {
            protocol_version,
            ..self
        }
    }

    pub fn with_invalid_block0_hash(self) -> Self {
        Self {
            invalid_block0_hash: true,
            ..self
        }
    }

    pub fn build(self) -> AdversaryNode {
        let data = MockBuilder::default()
            .with_invalid_block0_hash(self.invalid_block0_hash)
            .with_protocol_version(self.protocol_version)
            .with_genesis_block(self.genesis_block)
            .build_data();

        let controller = if self.server_enabled {
            Some(start_thread(data.clone()))
        } else {
            None
        };

        AdversaryNode::new(self.temp_dir, self.alias, data, controller)
    }
}
