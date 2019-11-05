//! module defining the p2p topology management objects
//!

use crate::network::p2p::{Gossips, Id, Node, Policy, PolicyConfig};
use poldercast::{
    poldercast::{Cyclon, Rings, Vicinity},
    Layer, NodeProfile, PolicyReport, StrikeReason, Topology,
};
use slog::Logger;
use std::sync::{Arc, RwLock};

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: Arc<RwLock<Topology>>,
    logger: Logger,
}

impl P2pTopology {
    /// create a new P2pTopology for the given Address and Id
    ///
    /// The address is the public
    pub fn new(node: poldercast::NodeProfile, logger: Logger) -> Self {
        P2pTopology {
            lock: Arc::new(RwLock::new(Topology::new(node))),
            logger,
        }
    }

    /// set a P2P Topology Module. Each module will work independently from
    /// each other and will help improve the node connectivity
    pub fn add_module<M: Layer + Send + Sync + 'static>(&self, module: M) {
        let mut topology = self.lock.write().unwrap();
        info!(
            self.logger,
            "adding P2P Topology module: {}",
            module.alias()
        );
        topology.add_layer(module)
    }

    pub fn set_policy(&mut self, policy: PolicyConfig) {
        let mut topology = self.lock.write().unwrap();
        topology.set_policy(Policy::new(policy, self.logger.new(o!("task" => "policy"))));
    }

    /// set all the default poldercast modules (Rings, Vicinity and Cyclon)
    pub fn set_poldercast_modules(&mut self) {
        let mut topology = self.lock.write().unwrap();
        topology.add_layer(Rings::default());
        topology.add_layer(Vicinity::default());
        topology.add_layer(Cyclon::default());
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub fn view(&self) -> Vec<Node> {
        let mut topology = self.lock.write().unwrap();
        topology
            .view(None, poldercast::Selection::Any)
            .into_iter()
            .map(Node::new)
            .collect()
    }

    pub fn initiate_gossips(&self, with: Id) -> Gossips {
        let mut topology = self.lock.write().unwrap();
        topology.initiate_gossips(with.into()).into()
    }

    pub fn accept_gossips(&self, from: Id, gossips: Gossips) {
        let mut topology = self.lock.write().unwrap();
        topology.accept_gossips(from.into(), gossips.into())
    }

    pub fn exchange_gossips(&mut self, with: Id, gossips: Gossips) -> Gossips {
        let mut topology = self.lock.write().unwrap();
        topology
            .exchange_gossips(with.into(), gossips.into())
            .into()
    }

    pub fn node(&self) -> NodeProfile {
        self.lock.read().unwrap().profile().clone()
    }

    /// register a strike against the given node id
    ///
    /// the function returns `None` if the node was not even in the
    /// the topology (not even quarantined).
    pub fn report_node(&self, node: Id, issue: StrikeReason) -> Option<PolicyReport> {
        let mut topology = self.lock.write().unwrap();
        topology.update_node(node.into(), |node| {
            node.record_mut().strike(issue);
        })
    }
}
