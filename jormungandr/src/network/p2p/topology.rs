//! module defining the p2p topology management objects
//!

use crate::{
    log::KEY_SUB_TASK,
    network::p2p::{Gossips, Id, Node, Policy, PolicyConfig},
    settings::start::network::Configuration,
};
use futures03::prelude::*;
use poldercast::{
    custom_layers,
    poldercast::{Cyclon, Rings, Vicinity},
    NodeProfile, PolicyReport, StrikeReason, Topology,
};
use slog::Logger;
use tokio02::sync::{Mutex, MutexGuard};

pub struct View {
    pub self_node: NodeProfile,
    pub peers: Vec<Node>,
}

/// object holding the P2pTopology of the Node
#[derive(Clone)]
pub struct P2pTopology {
    lock: Lock<Topology>,
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
        self
    }

    fn build(self) -> P2pTopology {
        let node_id = self.topology.profile().id().clone();
        P2pTopology {
            lock: Lock::new(self.topology),
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

    // TODO: same as write now, but can be implemented differently
    // with RwLock in tokio 0.2
    fn read<E>(&self) -> impl Future<Item = LockGuard<Topology>, Error = E> {
        self.write()
    }

    fn write<E>(&self) -> impl Future<Item = LockGuard<Topology>, Error = E> {
        let mut lock = self.lock.clone();
        future::poll_fn(move || Ok(lock.poll_lock()))
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub fn view<E>(&self, selection: poldercast::Selection) -> impl Future<Item = View, Error = E> {
        self.write().map(move |mut topology| {
            let peers = topology
                .view(None, selection)
                .into_iter()
                .map(Node::new)
                .collect();
            View {
                self_node: topology.profile().clone(),
                peers,
            }
        })
    }

    pub fn initiate_gossips<E>(&self, with: Id) -> impl Future<Item = Gossips, Error = E> {
        self.write()
            .map(move |mut topology| topology.initiate_gossips(with.into()).into())
    }

    pub fn accept_gossips<E>(
        &self,
        from: Id,
        gossips: Gossips,
    ) -> impl Future<Item = (), Error = E> {
        self.write()
            .map(move |mut topology| topology.accept_gossips(from.into(), gossips.into()))
    }

    pub fn exchange_gossips<E>(
        &mut self,
        with: Id,
        gossips: Gossips,
    ) -> impl Future<Item = Gossips, Error = E> {
        self.write().map(move |mut topology| {
            topology
                .exchange_gossips(with.into(), gossips.into())
                .into()
        })
    }

    pub fn node_id(&self) -> Id {
        self.node_id
    }

    pub fn node<E>(&self) -> impl Future<Item = NodeProfile, Error = E> {
        self.read().map(|topology| topology.profile().clone())
    }

    pub fn force_reset_layers<E>(&self) -> impl Future<Item = (), Error = E> {
        self.write()
            .map(|mut topology| topology.force_reset_layers())
    }

    pub fn list_quarantined<E>(&self) -> impl Future<Item = Vec<poldercast::Node>, Error = E> {
        self.read().map(|topology| {
            topology
                .nodes()
                .all_quarantined_nodes()
                .into_iter()
                .cloned()
                .collect()
        })
    }

    pub fn list_available<E>(&self) -> impl Future<Item = Vec<poldercast::Node>, Error = E> {
        self.read().map(|topology| {
            topology
                .nodes()
                .all_available_nodes()
                .into_iter()
                .cloned()
                .collect()
        })
    }

    pub fn list_non_public<E>(&self) -> impl Future<Item = Vec<poldercast::Node>, Error = E> {
        self.read().map(|topology| {
            topology
                .nodes()
                .all_unreachable_nodes()
                .into_iter()
                .cloned()
                .collect()
        })
    }

    pub fn nodes_count<E>(&self) -> impl Future<Item = poldercast::Count, Error = E> {
        self.read().map(|topology| topology.nodes().node_count())
    }

    /// register a strike against the given node id
    ///
    /// the function returns `None` if the node was not even in the
    /// the topology (not even quarantined).
    pub fn report_node<E>(
        &self,
        node: Id,
        issue: StrikeReason,
    ) -> impl Future<Item = Option<PolicyReport>, Error = E> {
        self.write().map(move |mut topology| {
            topology.update_node(node.into(), |node| {
                node.record_mut().strike(issue);
            })
        })
    }
}
