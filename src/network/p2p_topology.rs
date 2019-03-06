//! module defining the p2p topology management objects
//!

use bincode;
use chain_core::property;
use network_core::gossip;
use poldercast::topology::{Cyclon, Module, Rings, Topology, Vicinity};
use poldercast::Subscription;
pub use poldercast::{Address, Id, InterestLevel, Node};
use std::{
    collections::BTreeMap,
    error, fmt, io,
    sync::{Arc, RwLock},
    vec,
};

pub const NEW_TRANSACTIONS_TOPIC: u32 = 0u32;
pub const NEW_BLOCKS_TOPIC: u32 = 1u32;

#[derive(Debug, Clone)]
pub struct Gossip(pub Vec<Node>);

#[derive(Debug)]
pub enum Error {
    Encoding(Box<bincode::ErrorKind>),
    IO(io::Error),
}

/*
impl Error {
    pub fn new<E>(source: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            source: source.into(),
        }
    }
}
*/

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Encoding(source) => Some(source),
            Error::IO(source) => Some(source),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Encoding(_) => write!(f, "serialization error"),
            Error::IO(_) => write!(f, "io error"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<Box<bincode::ErrorKind>> for Error {
    fn from(e: Box<bincode::ErrorKind>) -> Self {
        Error::Encoding(e)
    }
}

pub fn from_node_id(node_id: &gossip::NodeId) -> Id {
    let bytes = node_id.to_bytes();
    let mut buf = [0; 16];
    buf[0..16].clone_from_slice(&bytes);
    Id::from(u128::from_be_bytes(buf))
}

pub fn to_node_id(id: &Id) -> gossip::NodeId {
    gossip::NodeId::from_slice(&id.as_u128().to_be_bytes()).unwrap()
}

impl gossip::Gossip for Gossip {
    type NodeId = Id;
    type Node = Node;

    fn from_nodes<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Self::Node>,
    {
        Gossip(iter.into_iter().collect())
    }
}

impl IntoIterator for Gossip {
    type Item = Node;
    type IntoIter = vec::IntoIter<Node>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// object holding the P2pTopology of the Node
#[derive(Clone)]
pub struct P2pTopology {
    topology: Arc<RwLock<Topology>>,
}

impl property::Serialize for Gossip {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        bincode::serialize_into(writer, &self.0).map_err(Error::Encoding)
    }
}

impl property::Deserialize for Gossip {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let iter: Vec<Node> = bincode::deserialize_from(reader).map_err(Error::Encoding)?;
        Ok(Gossip(iter))
    }
}

impl P2pTopology {
    /// create a new P2pTopology for the given Address and Id
    ///
    /// The address is the public
    pub fn new(node: Node) -> Self {
        P2pTopology {
            topology: Arc::new(RwLock::new(Topology::new(node))),
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
        self.topology.read().unwrap().view()
    }

    /// this is the function to utilise when we receive a gossip in order
    /// to update the P2P Topology internal state
    pub fn update(&mut self, new_nodes: BTreeMap<Id, Node>) {
        self.topology.write().unwrap().update(new_nodes)
    }

    /// this is the function to utilise in order to select gossips to share
    /// with a given node
    pub fn select_gossips(&mut self, gossip_recipient: &Node) -> BTreeMap<Id, Node> {
        self.topology
            .write()
            .unwrap()
            .select_gossips(gossip_recipient)
    }
}

pub fn add_transaction_subscription(node: &mut Node, interest_level: InterestLevel) {
    node.add_subscription(Subscription::new(
        NEW_TRANSACTIONS_TOPIC.into(),
        interest_level,
    ));
}

pub fn add_block_subscription(node: &mut Node, interest_level: InterestLevel) {
    node.add_subscription(Subscription::new(NEW_BLOCKS_TOPIC.into(), interest_level));
}
