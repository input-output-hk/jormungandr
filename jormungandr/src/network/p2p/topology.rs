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
use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::RwLock;

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

pub struct Node(poldercast::Node);

#[derive(Clone, Debug)]
pub struct NodeData(poldercast::NodeData);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct NodeId(pub poldercast::Id);

impl gossip::Node for NodeData {
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
    pub fn new(private_id: poldercast::PrivateId, address: Option<Address>) -> Self {
        Node(if let Some(address) = address {
            poldercast::Node::new_with(private_id, address)
        } else {
            poldercast::Node::new(private_id)
        })
    }

    pub fn data(&self) -> NodeData {
        NodeData(self.0.data().clone())
    }

    pub fn add_message_subscription(&mut self, interest_level: InterestLevel) {
        self.0
            .data_mut()
            .add_subscription(Subscription::new(NEW_MESSAGES_TOPIC.into(), interest_level));
    }

    pub fn add_block_subscription(&mut self, interest_level: InterestLevel) {
        self.0
            .data_mut()
            .add_subscription(Subscription::new(NEW_BLOCKS_TOPIC.into(), interest_level));
    }
}

impl NodeData {
    pub fn poldercast_address(&self) -> &Option<poldercast::Address> {
        self.0.address()
    }

    pub fn has_valid_address(&self) -> bool {
        let addr = match self.address() {
            None => return false,
            Some(addr) => addr,
        };

        match addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_unspecified() {
                    return false;
                }
                if ip.is_broadcast() {
                    return false;
                }
                if ip.is_multicast() {
                    return false;
                }
                if ip.is_documentation() {
                    return false;
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_unspecified() {
                    return false;
                }
                if ip.is_multicast() {
                    return false;
                }
            }
        }

        true
    }

    pub fn is_global(&self) -> bool {
        if !self.has_valid_address() {
            return false;
        }

        let addr = match self.address() {
            None => return false,
            Some(addr) => addr,
        };

        match addr.ip() {
            IpAddr::V4(ip) => {
                if ip.is_private() {
                    return false;
                }
                if ip.is_loopback() {
                    return false;
                }
                if ip.is_link_local() {
                    return false;
                }
            }
            IpAddr::V6(ip) => {
                if ip.is_loopback() {
                    return false;
                }
                // FIXME: add more tests when Ipv6Addr convenience methods
                // get stabilized:
                // https://github.com/rust-lang/rust/issues/27709
            }
        }

        true
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: RwLock<Topology>,
    logger: Logger,
}

impl property::Serialize for NodeData {
    type Error = Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        Ok(bincode::serialize_into(writer, &self.0)?)
    }
}

impl property::Deserialize for NodeData {
    type Error = Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        bincode::deserialize_from(reader)
            .map(NodeData)
            .map_err(Into::into)
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
    pub fn view(&self) -> impl Iterator<Item = NodeData> {
        let topology = self.lock.read().unwrap();
        topology.view().into_iter().map(NodeData)
    }

    /// this is the function to utilise when we receive a gossip in order
    /// to update the P2P Topology internal state
    pub fn update<I>(&self, new_nodes: I)
    where
        I: IntoIterator<Item = NodeData>,
    {
        let tree = new_nodes
            .into_iter()
            .map(|node| (node.id().0, node.0))
            .collect();
        self.update_tree(tree)
    }

    fn update_tree(&self, new_nodes: BTreeMap<poldercast::Id, poldercast::NodeData>) {
        // Poldercast API should be better than this
        debug!(self.logger, "updating P2P topology");
        self.lock.write().unwrap().update(new_nodes)
    }

    /// this is the function to utilise in order to select gossips to share
    /// with a given node
    pub fn select_gossips(&self, gossip_recipient: &NodeData) -> impl Iterator<Item = NodeData> {
        let mut topology = self.lock.write().unwrap();
        topology
            .select_gossips(&gossip_recipient.0)
            .into_iter()
            .map(|(_, v)| NodeData(v))
    }

    pub fn evict_node(&self, id: NodeId) {
        let mut topology = self.lock.write().unwrap();
        topology.evict_node(id.0);
    }

    pub fn node(&self) -> NodeData {
        NodeData(self.lock.read().unwrap().node().data().clone())
    }
}

pub mod modules {
    use poldercast::{topology::Module, Id, NodeData};
    use std::collections::BTreeMap;

    pub struct TrustedPeers {
        peers: Vec<NodeData>,
    }
    impl TrustedPeers {
        pub fn new_with<I>(nodes: I) -> Self
        where
            I: IntoIterator<Item = NodeData>,
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
        fn update(&mut self, _our_node: &NodeData, _known_nodes: &BTreeMap<Id, NodeData>) {
            // DO NOTHING
        }
        fn select_gossips(
            &self,
            _our_node: &NodeData,
            _gossip_recipient: &NodeData,
            _known_nodes: &BTreeMap<Id, NodeData>,
        ) -> BTreeMap<Id, NodeData> {
            // Never gossip about our trusted nodes, this could breach network
            // trust
            BTreeMap::new()
        }
        fn view(&self, known_nodes: &BTreeMap<Id, NodeData>, view: &mut BTreeMap<Id, NodeData>) {
            const MAX_TRUSTED_PEER_VIEW: usize = 4;
            use rand::seq::SliceRandom;

            let mut rng = rand::thread_rng();

            let mut peers = self.peers.clone();
            peers.shuffle(&mut rng);

            for peer in peers.into_iter().take(MAX_TRUSTED_PEER_VIEW) {
                // if we received a gossip from the node, then prefer taking the node
                // with the appropriate node data as it may have updated its IP address
                // as received from a gossip
                let peer = if let Some(known_peer) = known_nodes.get(peer.id()) {
                    known_peer.clone()
                } else {
                    // otherwise use the peer from the trusted list
                    peer
                };

                // insert only if not already present
                view.entry(*peer.id()).or_insert(peer);
            }
        }
    }
}
