//! module defining the p2p topology management objects
//!
use super::{
    layers::{self, LayersConfig},
    quarantine::ReportNodeStatus,
    topic, Gossips, NodeId, Peer, PeerInfo, ReportRecords,
};
use crate::{
    metrics::{Metrics, MetricsBackend},
    settings::start::network::Configuration,
};
use chain_crypto::Ed25519;
use jormungandr_lib::crypto::key::SigningKey;
use poldercast::{
    layer::{self as poldercast_layer, Layer, LayerBuilder},
    Profile, Topology,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    convert::TryInto,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
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
    pub self_node: Peer,
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    topology: Topology,
    quarantine: ReportRecords,
    key: keynesis::key::ed25519::SecretKey,
    stats_counter: Metrics,
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
    fn build_layers(&self, rings: u8, _vicinity: usize, cyclon: usize) -> Vec<Box<dyn Layer>> {
        let mut layers: Vec<Box<dyn Layer>> = Vec::with_capacity(4);

        layers.push(Box::new(layers::Rings::new(
            self.config.rings.clone(),
            poldercast_layer::Rings::new(rings),
        )));
        // disabled until https://github.com/primetype/poldercast/pull/36 is fixed and merged
        //layers.push(Box::new(poldercast_layer::Vicinity::new(vicinity)));
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
    pub fn new(config: &Configuration, stats_counter: Metrics) -> Self {
        let addr = config.public_address.unwrap_or(*LOCAL_ADDR);
        let key = secret_key_into_keynesis(config.node_key.clone());

        let quarantine = ReportRecords::from_config(config.policy.clone());
        let custom_builder = CustomLayerBuilder::from(config.layers.clone());
        let mut topology = Topology::new_with(addr, &key, custom_builder);
        topology.subscribe_topic(topic::MESSAGES);
        topology.subscribe_topic(topic::BLOCKS);
        P2pTopology {
            topology,
            quarantine,
            key,
            stats_counter,
        }
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub fn view(&mut self, selection: poldercast::layer::Selection) -> View {
        let peers = self
            .topology
            .view(None, selection)
            .into_iter()
            .map(|profile| Peer::from(profile.gossip().clone()))
            .collect();
        View {
            peers,
            self_node: self.topology.self_profile().gossip().clone().into(),
        }
    }

    pub fn initiate_gossips(&mut self, recipient: &NodeId) -> Gossips {
        let mut gossips = self.topology.gossips_for(recipient.as_ref());
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
        let gossips = <Vec<poldercast::Gossip>>::from(gossips);
        for gossip in gossips {
            let peer = Profile::from_gossip(gossip);
            let peer_id = NodeId(peer.id());
            tracing::trace!(addr = %peer.address(), %peer_id, "received peer from incoming gossip");
            if self.topology.add_peer(peer) {
                self.quarantine.record_new_gossip(&peer_id);
                self.stats_counter
                    .set_peer_available_cnt(self.peer_available_cnt());
            }
        }
    }

    // This may return nodes that are still quarantined but have been
    // forgotten by the underlying poldercast implementation.
    pub fn list_quarantined(&self) -> Vec<PeerInfo> {
        let ids = self.topology.peers().dirty();
        // reported nodes also include reports against nodes that are not in the dirty pool
        // and we should include those here
        self.quarantine
            .reported_nodes()
            .into_iter()
            .filter(|profile| ids.contains(profile.id.as_ref()))
            .collect()
    }

    /// This returns the peers known to the node which are not quarantined.
    /// Please note some of these may not be present in the topology view.
    pub fn list_available(&self) -> impl Iterator<Item = Peer> + '_ {
        let profiles = self.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .map(|(_, profile)| profile.gossip().clone().into())
    }

    pub fn list_non_public(&self) -> impl Iterator<Item = Peer> + '_ {
        let profiles = self.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .filter_map(|(_, profile)| {
                let peer = Peer::from(profile.gossip().clone());
                if peer.is_global() {
                    None
                } else {
                    Some(peer)
                }
            })
    }

    /// register that we were able to establish an handshake with given peer
    pub fn promote_node(&mut self, node: &NodeId) {
        self.topology.promote_peer(node.as_ref());
        self.stats_counter
            .set_peer_available_cnt(self.peer_available_cnt());
    }

    /// register a strike against the given peer
    #[instrument(skip_all, level = "debug", fields(%node_id))]
    pub fn report_node(&mut self, node_id: &NodeId) {
        if let Some(node) = self.topology.get(node_id.as_ref()).cloned() {
            let result = self
                .quarantine
                .report_node(&mut self.topology, Peer::from(node.gossip().clone()));
            if let ReportNodeStatus::Quarantine | ReportNodeStatus::SoftReport = result {
                self.stats_counter
                    .set_peer_available_cnt(self.peer_available_cnt());
            }
            if let ReportNodeStatus::Quarantine = result {
                self.stats_counter.add_peer_quarantined_cnt(1);
            }
        }
    }

    /// update our gossip so that other nodes can see that we are updating
    /// it and are alive
    pub fn update_gossip(&mut self) {
        self.topology.update_profile_subscriptions(&self.key);
    }

    pub fn lift_reports(&mut self) -> Vec<Peer> {
        self.quarantine
            .lift_reports()
            .into_iter()
            .filter_map(|node| {
                let node = self.topology.peers().dirty().peek(node.id.as_ref()).cloned();
                // It may happen that a node is evicted from the dirty pool
                // in poldercast and then re-enters the topology in the 'pool'
                // pool, all while we hold the node in quarantine.
                // If that happens we should not promote it anymore.
                if let Some(node) = &node {
                    tracing::debug!(node = %node.address(), id=?node.id(), "lifting node from quarantine");
                    self.topology.promote_peer(&node.id());
                    self.stats_counter.sub_peer_quarantined_cnt(1);
                    self.stats_counter
                    .set_peer_available_cnt(self.peer_available_cnt());
                }
                node.map(|node| Peer::from(node.gossip().clone()))
            })
            .collect()
    }

    fn peer_available_cnt(&self) -> usize {
        // We cannot use ExactSizeIterator as a limitation of iterator::chain, but since
        // size_hint still relies on the underlying exact size iterator, it is equivalent.
        self.list_available().size_hint().0
    }
}
