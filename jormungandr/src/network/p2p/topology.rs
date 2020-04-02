//! module defining the p2p topology management objects
//!

use crate::{
    log::KEY_SUB_TASK,
    network::p2p::{layers::PreferredListLayer, Gossips, Id, Node, Policy, PolicyConfig},
    settings::start::network::Configuration,
};
use futures03::prelude::*;
use poldercast::{
    custom_layers,
    poldercast::{Cyclon, Rings, Vicinity},
    NodeProfile, PolicyReport, StrikeReason, Topology,
};
use slog::Logger;
use tokio02::sync::RwLock;

pub struct View {
    pub self_node: NodeProfile,
    pub peers: Vec<Node>,
}

/// object holding the P2pTopology of the Node
#[derive(Clone)]
pub struct P2pTopology {
    lock: RwLock<Topology>,
    node_id: Id,
    logger: Logger,
}

/// Builder object used to initialize the `P2pTopology`
struct Builder {
    topology: Topology,
    logger: Logger,
}

impl Builder {
    /// Create a new topology for the given node profile
    fn new(node: poldercast::NodeProfile, logger: Logger) -> Self {
        Builder {
            topology: Topology::new(node),
            logger,
        }
    }

    fn set_policy(mut self, policy: PolicyConfig) -> Self {
        self.topology.set_policy(Policy::new(
            policy,
            self.logger.new(o!(KEY_SUB_TASK => "policy")),
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

    fn set_custom_modules(mut self, config: &Configuration) -> Self {
        if let Some(size) = config.max_unreachable_nodes_to_connect_per_event {
            self.topology
                .add_layer(custom_layers::RandomDirectConnections::with_max_view_length(size));
        } else {
            self.topology
                .add_layer(custom_layers::RandomDirectConnections::default());
        }

        self.topology.add_layer(PreferredListLayer::new(
            config.layers.preferred_list.clone(),
        ));

        self
    }

    fn build(self) -> P2pTopology {
        let node_id = self.topology.profile().id().clone();
        P2pTopology {
            lock: RwLock::new(self.topology),
            node_id: node_id.into(),
            logger: self.logger,
        }
    }
}

impl P2pTopology {
    pub fn new(config: &Configuration, logger: Logger) -> Self {
        Builder::new(config.profile.clone(), logger)
            .set_poldercast_modules()
            .set_custom_modules(&config)
            .set_policy(config.policy.clone())
            .build()
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub async fn view(&self, selection: poldercast::Selection) -> View {
        let topology = self.lock.write().await;
        let peers = topology
            .view(None, selection)
            .into_iter()
            .map(Node::new)
            .collect();
        View {
            self_node: topology.profile().clone(),
            peers,
        }
    }

    pub async fn initiate_gossips(&self, with: Id) -> Gossips {
        let topology = self.lock.write().await;
        topology.initiate_gossips(with.into()).into()
    }

    pub async fn accept_gossips(&self, from: Id, gossips: Gossips) {
        let topology = self.lock.write().await;
        topology.accept_gossips(from.into(), gossips.into())
    }

    pub async fn exchange_gossips(&mut self, with: Id, gossips: Gossips) -> Gossips {
        let topology = self.lock.write().await;
        topology
            .exchange_gossips(with.into(), gossips.into())
            .into()
    }

    pub fn node_id(&self) -> Id {
        self.node_id
    }

    pub async fn node(&self) -> NodeProfile {
        let topology = self.lock.read().await;
        topology.profile().clone()
    }

    pub async fn force_reset_layers(&self) {
        let topology = self.lock.write().await;
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
    pub async fn report_node(&self, node: Id, issue: StrikeReason) -> Option<PolicyReport> {
        let topology = self.lock.write().await;
        topology.update_node(node.into(), |node| {
            node.record_mut().strike(issue);
        })
    }
}
