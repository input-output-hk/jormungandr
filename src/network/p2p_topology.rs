//! module defining the p2p topology management objects
//!

use bincode;
use chain_core::property;
use network_core::gossip::{self, Node as _};
use poldercast::topology::{Cyclon, Module, Rings, Topology, Vicinity};
use poldercast::Subscription;
pub use poldercast::{Address, InterestLevel};
use std::{
    collections::BTreeMap,
    error, fmt, io,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

pub const NEW_MESSAGES_TOPIC: u32 = 0u32;
pub const NEW_BLOCKS_TOPIC: u32 = 1u32;

#[derive(Debug)]
pub enum Error {
    Encoding(Box<bincode::ErrorKind>),
    Io(io::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Encoding(source) => Some(source),
            Error::Io(source) => Some(source),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Encoding(_) => write!(f, "serialization error"),
            Error::Io(_) => write!(f, "io error"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<Box<bincode::ErrorKind>> for Error {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        Error::Encoding(e)
    }
}

#[derive(Clone, Debug)]
pub struct Node(poldercast::Node);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct NodeId(poldercast::Id);

impl gossip::Node for Node {
    type Id = NodeId;

    #[inline]
    fn id(&self) -> Self::Id {
        NodeId(self.0.id().clone())
    }

    #[inline]
    fn address(&self) -> Option<SocketAddr> {
        self.0.address().to_socketaddr()
    }
}

impl gossip::NodeId for NodeId {}

impl Node {
    #[inline]
    pub fn new(id: NodeId, address: Address) -> Self {
        Node(poldercast::Node::new(id.0, address))
    }

    pub fn add_message_subscription(&mut self, interest_level: InterestLevel) {
        self.0
            .add_subscription(Subscription::new(NEW_MESSAGES_TOPIC.into(), interest_level));
    }

    pub fn add_block_subscription(&mut self, interest_level: InterestLevel) {
        self.0
            .add_subscription(Subscription::new(NEW_BLOCKS_TOPIC.into(), interest_level));
    }
}

impl NodeId {
    #[inline]
    pub fn generate() -> Self {
        NodeId(poldercast::Id::generate(&mut rand::thread_rng()))
    }
}

/// object holding the P2pTopology of the Node
#[derive(Clone)]
pub struct P2pTopology {
    topology: Arc<RwLock<Topology>>,
}

impl property::Serialize for Node {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, &self.0).map_err(Error::Encoding)
    }
}

impl property::Deserialize for Node {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let inner = bincode::deserialize_from(reader).map_err(Error::Encoding)?;
        Ok(Node(inner))
    }
}

impl property::Serialize for NodeId {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, &self.0).map_err(Error::Encoding)
    }
}

impl property::Deserialize for NodeId {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let id = bincode::deserialize_from(reader).map_err(Error::Encoding)?;
        Ok(NodeId(id))
    }
}

impl P2pTopology {
    /// create a new P2pTopology for the given Address and Id
    ///
    /// The address is the public
    pub fn new(node: Node) -> Self {
        P2pTopology {
            topology: Arc::new(RwLock::new(Topology::new(node.0))),
        }
    }

    /// set a P2P Topology Module. Each module will work independently from
    /// each other and will help improve the node connectivity
    pub fn add_module<M: Module + Send + Sync + 'static>(&mut self, module: M) {
        info!("setting P2P Topology module: {}", module.name());
        self.topology.write().unwrap().add_module(module)
    }

    /// set all the default poldercast modules (Rings, Vicinity and Cyclon)
    pub fn set_poldercast_modules(&mut self) {
        let mut topology = self.topology.write().unwrap();
        topology.add_module(Rings::new());
        topology.add_module(Vicinity::new());
        topology.add_module(Cyclon::new());
    }

    /// this is the list of neighbors to contact for event dissemination
    pub fn view(&self) -> Vec<Node> {
        let topology = self.topology.read().unwrap();
        topology.view().into_iter().map(Node).collect()
    }

    /// this is the function to utilise when we receive a gossip in order
    /// to update the P2P Topology internal state
    pub fn update<I>(&mut self, new_nodes: I)
    where
        I: IntoIterator<Item = Node>,
    {
        let tree = new_nodes
            .into_iter()
            .map(|node| (node.id().0, node.0))
            .collect();
        self.update_tree(tree)
    }

    fn update_tree(&mut self, new_nodes: BTreeMap<poldercast::Id, poldercast::Node>) {
        // Poldercast API should be better than this
        self.topology.write().unwrap().update(new_nodes)
    }

    /// this is the function to utilise in order to select gossips to share
    /// with a given node
    pub fn select_gossips(&mut self, gossip_recipient: &Node) -> impl Iterator<Item = Node> {
        let mut topology = self.topology.write().unwrap();
        topology
            .select_gossips(&gossip_recipient.0)
            .into_iter()
            .map(|(_, v)| Node(v))
    }
}
