//! module defining the p2p topology management objects
//!
use super::{
    layers::{self, LayersConfig},
    topic, Gossip, Gossips, NodeId, Peer, PeerInfo, Quarantine,
};

use crate::settings::start::network::Configuration;
use chain_crypto::Ed25519;
use jormungandr_lib::crypto::key::SigningKey;
use poldercast::{
    layer::{self as poldercast_layer, Layer, LayerBuilder},
    Profile, Topology,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tracing::instrument;

lazy_static! {
    static ref LOCAL_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
}

pub fn secret_key_into_keynesis(key: SigningKey<Ed25519>) -> keynesis::key::ed25519::SecretKey {
    let key_bytes = key.into_secret_key().leak_secret();
    key_bytes.as_ref().try_into().unwrap()
}

pub struct View {
    pub peers: Vec<Peer>,
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    topology: Topology,
    quarantine: Quarantine,
}

struct CustomLayerBuilder {
    config: LayersConfig,
}

impl CustomLayerBuilder {
    // Default values from poldercast
    const RINGS_VIEW_SIZE: u8 = 4;
    const VICINITY_VIEW_SIZE: usize = 20;
    const CYCLON_VIEW_SIZE: usize = 20;
    const GOSSIP_SIZE: u8 = 10;
}

impl From<LayersConfig> for CustomLayerBuilder {
    fn from(config: LayersConfig) -> Self {
        Self { config }
    }
}

impl CustomLayerBuilder {
    fn build_layers(&self, rings: u8, vicinity: usize, cyclon: usize) -> Vec<Box<dyn Layer>> {
        let mut layers: Vec<Box<dyn Layer>> = Vec::with_capacity(4);

        layers.push(Box::new(layers::Rings::new(
            self.config.rings.clone(),
            poldercast_layer::Rings::new(rings),
        )));
        layers.push(Box::new(poldercast_layer::Vicinity::new(vicinity)));
        layers.push(Box::new(poldercast_layer::Cyclon::new(cyclon)));

        let mut seed = [0; 32];
        rand::thread_rng().fill(&mut seed);
        layers.push(Box::new(layers::PreferredListLayer::new(
            &self.config.preferred_list,
            ChaChaRng::from_seed(seed),
        )));

        layers
    }
}

impl LayerBuilder for CustomLayerBuilder {
    fn build_for_view(&self) -> Vec<Box<dyn Layer>> {
        self.build_layers(
            Self::RINGS_VIEW_SIZE,
            Self::VICINITY_VIEW_SIZE,
            Self::CYCLON_VIEW_SIZE,
        )
    }

    fn build_for_gossip(&self) -> Vec<Box<dyn Layer>> {
        self.build_layers(
            Self::GOSSIP_SIZE,
            Self::GOSSIP_SIZE.into(),
            Self::GOSSIP_SIZE.into(),
        )
    }
}

impl P2pTopology {
    pub fn new(config: &Configuration) -> Self {
        let addr = config.public_address.or(Some(*LOCAL_ADDR)).unwrap();
        let key = secret_key_into_keynesis(config.node_key.clone());

        let quarantine = Quarantine::from_config(config.policy.clone());
        let custom_builder = CustomLayerBuilder::from(config.layers.clone());
        let mut topology = Topology::new_with(addr, &key, custom_builder);
        topology.subscribe_topic(topic::MESSAGES);
        topology.subscribe_topic(topic::BLOCKS);
        P2pTopology {
            topology,
            quarantine,
        }
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub fn view(&mut self, selection: poldercast::layer::Selection) -> View {
        let peers = self
            .topology
            .view(None, selection)
            .into_iter()
            .map(|profile| Peer {
                addr: profile.address(),
                id: Some(NodeId(profile.id())),
            })
            .collect();
        View { peers }
    }

    // If the recipient is not specified gossip will only contain information
    // about this node
    pub fn initiate_gossips(&mut self, recipient: Option<&NodeId>) -> Gossips {
        let mut gossips = if let Some(recipient) = recipient {
            self.topology.gossips_for(recipient.as_ref())
        } else {
            Vec::new()
        };
        // If the recipient is not already in the topology
        // or was not specified poldercast will not return anything.
        // Let's broadcast out profile anyway
        if gossips.is_empty() {
            gossips.push(self.topology.self_profile().gossip().clone());
        }
        gossips.retain(|g| g.address() != *LOCAL_ADDR);
        Gossips::from(gossips)
    }

    #[instrument(skip(self, gossips), level = "debug")]
    pub fn accept_gossips(&mut self, gossips: Gossips) {
        // Even if lifted from quarantine, peers will be re-added to the topology
        // only after we receive a gossip about them.
        let lifted = self.quarantine.lift_from_quarantine();
        for node in lifted {
            // It may happen that a node is evicted from the dirty pool
            // in poldercast and then re-enters the topology in the 'pool'
            // pool, all while we hold the node in quarantine.
            // If that happens we should not promote it anymore.
            let is_dirty = self.topology.peers().dirty().contains(node.id.as_ref());
            if is_dirty {
                tracing::debug!(node = %node.address, "lifting node from quarantine");
                self.topology.promote_peer(&node.id.as_ref());
            } else {
                tracing::debug!(node = %node.address, "node from quarantine have left the dirty pool. skipping it");
            }
        }

        let gossips = <Vec<poldercast::Gossip>>::from(gossips);
        for gossip in gossips {
            let peer = Profile::from_gossip(gossip);
            tracing::trace!(node = %peer.address(), "received peer from gossip");
            self.topology.add_peer(peer);
        }
    }

    // This may return nodes that are still quarantined but have been
    // forgotten by the underlying poldercast implementation.
    pub fn list_quarantined(&self) -> Vec<PeerInfo> {
        self.quarantine.quarantined_nodes()
    }

    pub fn list_available(&self) -> Vec<PeerInfo> {
        let profiles = self.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .map(|(_, profile)| profile.into())
            .collect()
    }

    pub fn list_non_public(&self) -> Vec<PeerInfo> {
        let profiles = self.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .filter_map(|(_, profile)| {
                if Gossip::from(profile.gossip().clone()).is_global() {
                    None
                } else {
                    Some(profile.into())
                }
            })
            .collect()
    }

    /// register that we were able to establish an handshake with given peer
    pub fn promote_node(&mut self, node: &NodeId) {
        self.topology.promote_peer(node.as_ref());
    }

    /// register a strike against the given peer
    pub fn report_node(&mut self, node_id: &NodeId) {
        if let Some(node) = self.topology.get(node_id.as_ref()).cloned() {
            if self.quarantine.quarantine_node((&node).into()) {
                self.topology.remove_peer(node_id.as_ref());
                // Trusted peers in poldercast requires to be demoted 2 times before
                // moving to the dirty pool
                self.topology.remove_peer(node_id.as_ref());
            }
        }
    }
}
