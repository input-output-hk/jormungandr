//! module defining the p2p topology management objects
//!

use crate::{
    network::p2p::{layers::PreferredListLayer, Address, Gossips, Policy, PolicyConfig},
    settings::start::network::Configuration,
};
use poldercast::{
    custom_layers,
    poldercast::{Cyclon, Rings, Vicinity},
    NodeProfile, PolicyReport, StrikeReason, Topology,
};
use rand_chacha::ChaChaRng;
use tokio::sync::RwLock;
use tracing::{span, Level, Span};

pub struct View {
    pub self_node: NodeProfile,
    pub peers: Vec<Address>,
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: RwLock<Topology>,
}

/// Builder object used to initialize the `P2pTopology`
struct Builder {
    topology: Topology,
    span: Span,
}

impl Builder {
    /// Create a new topology for the given node profile
    fn new(node: poldercast::NodeProfile, span: Span) -> Self {
        Builder {
            topology: Topology::new(node),
            span,
        }
    }

    fn set_policy(mut self, policy: PolicyConfig) -> Self {
        self.topology.set_policy(Policy::new(
            policy,
            span!(parent: &self.span, Level::TRACE, "sub_task", name = "policy"),
        ));
        self
    }

    /// set all the default poldercast modules (Rings, Vicinity and Cyclon)
    fn set_poldercast_modules(mut self) -> Self {
        self.topology.add_layer(Rings::default());
        self.topology.add_layer(Vicinity::default());
        self.topology.add_layer(Cyclon::default());
        self
    }

    fn set_custom_modules(mut self, config: &Configuration, rng: ChaChaRng) -> Self {
        if let Some(size) = config.max_unreachable_nodes_to_connect_per_event {
            self.topology
                .add_layer(custom_layers::RandomDirectConnections::with_max_view_length(size));
        } else {
            self.topology
                .add_layer(custom_layers::RandomDirectConnections::default());
        }

        self.topology.add_layer(PreferredListLayer::new(
            config.layers.preferred_list.clone(),
            rng,
        ));

        self
    }

    fn build(self) -> P2pTopology {
        P2pTopology {
            lock: RwLock::new(self.topology),
        }
    }
}

impl P2pTopology {
    pub fn new(config: &Configuration, span: Span, rng: ChaChaRng) -> Self {
        Builder::new(config.profile.clone(), span)
            .set_poldercast_modules()
            .set_custom_modules(&config, rng)
            .set_policy(config.policy.clone())
            .build()
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub async fn view(&self, selection: poldercast::Selection) -> View {
        let mut topology = self.lock.write().await;
        let peers = topology.view(None, selection).into_iter().collect();
        View {
            self_node: topology.profile().clone(),
            peers,
        }
    }

    pub async fn initiate_gossips(&self, with: Address) -> Gossips {
        let mut topology = self.lock.write().await;
        topology.initiate_gossips(with).into()
    }

    pub async fn accept_gossips(&self, from: Address, gossips: Gossips) {
        let mut topology = self.lock.write().await;
        topology.accept_gossips(from, gossips.into())
    }

    pub async fn exchange_gossips(&mut self, with: Address, gossips: Gossips) -> Gossips {
        let mut topology = self.lock.write().await;
        topology.exchange_gossips(with, gossips.into()).into()
    }

    pub async fn node(&self) -> NodeProfile {
        let topology = self.lock.read().await;
        topology.profile().clone()
    }

    pub async fn force_reset_layers(&self) {
        let mut topology = self.lock.write().await;
        topology.force_reset_layers()
    }

    pub async fn list_quarantined(&self) -> Vec<poldercast::Node> {
        let topology = self.lock.read().await;
        topology
            .nodes()
            .all_quarantined_nodes()
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn list_available(&self) -> Vec<poldercast::Node> {
        let topology = self.lock.read().await;
        topology
            .nodes()
            .all_available_nodes()
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn list_non_public(&self) -> Vec<poldercast::Node> {
        let topology = self.lock.read().await;
        topology
            .nodes()
            .all_unreachable_nodes()
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn nodes_count(&self) -> poldercast::Count {
        let topology = self.lock.read().await;
        topology.nodes().node_count()
    }

    /// register a strike against the given node id
    ///
    /// the function returns `None` if the node was not even in the
    /// the topology (not even quarantined).
    pub async fn report_node(&self, address: Address, issue: StrikeReason) -> Option<PolicyReport> {
        let mut topology = self.lock.write().await;
        topology.update_node(address, |node| {
            node.record_mut().strike(issue);
        })
    }
}
