//! module defining the p2p topology management objects
//!

use bincode;
use chain_core::property;
use network_core::gossip::{self, Node as _};
use poldercast::topology::{Cyclon, Module, Rings, Topology, Vicinity};
use poldercast::Subscription;
pub use poldercast::{Address, InterestLevel};
use serde::{Deserialize, Serialize};
use slog::Logger;
use std::{collections::BTreeMap, fmt, io, net::SocketAddr, sync::RwLock};

pub const NEW_MESSAGES_TOPIC: u32 = 0u32;
pub const NEW_BLOCKS_TOPIC: u32 = 1u32;

custom_error! {pub Error
    Encoding { source: bincode::ErrorKind } = "Serialization error",
    Io { source: io::Error } = "I/O Error",
}

impl From<bincode::Error> for Error {
    fn from(source: bincode::Error) -> Self {
        Error::Encoding { source: *source }
    }
}

#[derive(Clone, Debug)]
pub struct Node(poldercast::Node);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct NodeId(pub poldercast::Id);

impl gossip::Node for Node {
    type Id = NodeId;

    #[inline]
    fn id(&self) -> Self::Id {
        NodeId(self.0.id().clone())
    }

    #[inline]
    fn address(&self) -> Option<SocketAddr> {
        if let Some(address) = self.0.address() {
            address.to_socketaddr()
        } else {
            None
        }
    }
}

impl gossip::NodeId for NodeId {}

impl Node {
    #[inline]
    pub fn new(address: Option<Address>) -> Self {
        if let Some(address) = address {
            Node(poldercast::Node::new_with(address))
        } else {
            Node(poldercast::Node::new(
                &mut rand::rngs::OsRng::new().unwrap(),
            ))
        }
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

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", u64::from(self.0))
    }
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: RwLock<Topology>,
    logger: Logger,
}

impl property::Serialize for Node {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        Ok(bincode::serialize_into(writer, &self.0)?)
    }
}

impl property::Deserialize for Node {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let inner = bincode::deserialize_from(reader)?;
        Ok(Node(inner))
    }
}

impl property::Serialize for NodeId {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        Ok(bincode::serialize_into(writer, &self.0)?)
    }
}

impl property::Deserialize for NodeId {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let id = bincode::deserialize_from(reader)?;
        Ok(NodeId(id))
    }
}

impl P2pTopology {
    /// create a new P2pTopology for the given Address and Id
    ///
    /// The address is the public
    pub fn new(node: Node, logger: Logger) -> Self {
        P2pTopology {
            lock: RwLock::new(Topology::new(node.0)),
            logger,
        }
    }

    /// set a P2P Topology Module. Each module will work independently from
    /// each other and will help improve the node connectivity
    pub fn add_module<M: Module + Send + Sync + 'static>(&self, module: M) {
        let mut topology = self.lock.write().unwrap();
        info!(self.logger, "adding P2P Topology module: {}", module.name());
        topology.add_module(module)
    }

    /// set all the default poldercast modules (Rings, Vicinity and Cyclon)
    pub fn set_poldercast_modules(&mut self) {
        let mut topology = self.lock.write().unwrap();
        topology.add_module(Rings::default());
        topology.add_module(Vicinity::default());
        topology.add_module(Cyclon::default());
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub fn view(&self) -> impl Iterator<Item = Node> {
        let topology = self.lock.read().unwrap();
        topology.view().into_iter().map(Node)
    }

    /// this is the function to utilise when we receive a gossip in order
    /// to update the P2P Topology internal state
    pub fn update<I>(&self, new_nodes: I)
    where
        I: IntoIterator<Item = Node>,
    {
        let tree = new_nodes
            .into_iter()
            .map(|node| (node.id().0, node.0))
            .collect();
        self.update_tree(tree)
    }

    fn update_tree(&self, new_nodes: BTreeMap<poldercast::Id, poldercast::Node>) {
        // Poldercast API should be better than this
        debug!(self.logger, "updating P2P local topology");
        self.lock.write().unwrap().update(new_nodes)
    }

    /// this is the function to utilise in order to select gossips to share
    /// with a given node
    pub fn select_gossips(&self, gossip_recipient: &Node) -> impl Iterator<Item = Node> {
        debug!(
            self.logger,
            "selecting gossips for {}",
            gossip_recipient.id()
        );
        let mut topology = self.lock.write().unwrap();
        topology
            .select_gossips(&gossip_recipient.0)
            .into_iter()
            .map(|(_, v)| Node(v))
    }
}

pub mod modules {
    use poldercast::{topology::Module, Id, Node};
    use std::collections::BTreeMap;

    pub struct TrustedPeers {
        peers: Vec<Node>,
    }
    impl TrustedPeers {
        pub fn new_with<I>(nodes: I) -> Self
        where
            I: IntoIterator<Item = Node>,
        {
            TrustedPeers {
                peers: nodes.into_iter().collect(),
            }
        }
    }

    impl Module for TrustedPeers {
        fn name(&self) -> &'static str {
            "trusted-peers"
        }
        fn update(&mut self, _our_node: &Node, _known_nodes: &BTreeMap<Id, Node>) {
            // DO NOTHING
        }
        fn select_gossips(
            &self,
            _our_node: &Node,
            _gossip_recipient: &Node,
            _known_nodes: &BTreeMap<Id, Node>,
        ) -> BTreeMap<Id, Node> {
            // Never gossip about our trusted nodes, this could breach network
            // trust
            BTreeMap::new()
        }
        fn view(&self, _: &BTreeMap<Id, Node>, view: &mut BTreeMap<Id, Node>) {
            view.extend(self.peers.iter().map(|node| (*node.id(), node.clone())))
        }
    }
}
