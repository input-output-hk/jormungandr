//! module defining the p2p topology management objects
//!
use super::{topic, Gossip, Gossips, NodeId, Peer, PeerInfo, Quarantine};

use crate::settings::start::network::Configuration;
use chain_crypto::Ed25519;
use jormungandr_lib::crypto::key::SigningKey;
use poldercast::{Profile, Topology};
use std::convert::TryInto;
use tokio::sync::RwLock;
use tracing::instrument;

pub fn secret_key_into_keynesis(key: SigningKey<Ed25519>) -> keynesis::key::ed25519::SecretKey {
    let key_bytes = key.into_secret_key().leak_secret();
    key_bytes.as_ref().try_into().unwrap()
}

pub struct View {
    pub peers: Vec<Peer>,
}

struct Inner {
    topology: Topology,
    quarantine: Quarantine,
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: RwLock<Inner>,
}

impl P2pTopology {
    pub fn new(config: &Configuration) -> Self {
        // FIXME: How should we handle cases where the is not listen set? Can a node just receive?
        let addr = config.public_address.unwrap();
        let key = secret_key_into_keynesis(config.node_key.clone());

        let quarantine = Quarantine::from_config(config.policy.clone());
        let mut topology = Topology::new(addr, &key);
        topology.subscribe_topic(topic::MESSAGES);
        topology.subscribe_topic(topic::BLOCKS);
        let inner = Inner {
            topology,
            quarantine,
        };
        P2pTopology {
            lock: RwLock::new(inner),
        }
    }

    /// Returns a list of neighbors selected in this turn
    /// to contact for event dissemination.
    pub async fn view(&self, selection: poldercast::layer::Selection) -> View {
        let mut inner = self.lock.write().await;
        let peers = inner
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
    pub async fn initiate_gossips(&self, recipient: Option<&NodeId>) -> Gossips {
        let mut inner = self.lock.write().await;
        let mut gossips = if let Some(recipient) = recipient {
            inner.topology.gossips_for(recipient.as_ref())
        } else {
            Vec::new()
        };
        // If the recipient is not already in the topology
        // or was not specified poldercast will not return anything.
        // Let's broadcast out profile anyway
        if gossips.is_empty() {
            gossips.push(inner.topology.self_profile().gossip().clone());
        }
        Gossips::from(gossips)
    }

    #[instrument(skip(self, gossips), level = "debug")]
    pub async fn accept_gossips(&self, gossips: Gossips) {
        let mut inner = self.lock.write().await;
        let gossips = <Vec<poldercast::Gossip>>::from(gossips);
        for gossip in gossips {
            let peer = Profile::from_gossip(gossip);
            tracing::trace!(node = %peer.address(), "received peer from gossip");
            inner.topology.add_peer(peer);
        }

        // nodes lifted from quarantine will be considered again in the next update
        let lifted = inner.quarantine.lift_from_quarantine();
        for node in lifted {
            // It may happen that a node is evicted from the dirty pool
            // in poldercast and then re-enters the topology in the 'pool'
            // pool, all while we hold the node in quarantine.
            // If that happens we should not promote it anymore.
            let is_dirty = inner.topology.peers().dirty().contains(node.id.as_ref());
            if is_dirty {
                tracing::debug!(node = %node.address, "lifting node from quarantine");
                inner.topology.promote_peer(&node.id.as_ref());
            } else {
                tracing::debug!(node = %node.address, "node from quarantine have left the dirty pool. skipping it");
            }
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn force_reset_layers(&self) {
        tracing::warn!("resetting layers is not supported in this poldercast version");
    }

    // This may return nodes that are still quarantined but have been
    // forgotten by the underlying poldercast implementation.
    pub async fn list_quarantined(&self) -> Vec<PeerInfo> {
        self.lock.read().await.quarantine.quarantined_nodes()
    }

    pub async fn list_available(&self) -> Vec<PeerInfo> {
        let inner = self.lock.read().await;
        let profiles = inner.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .map(|(_, profile)| profile.into())
            .collect()
    }

    pub async fn list_non_public(&self) -> Vec<PeerInfo> {
        let inner = self.lock.read().await;
        let profiles = inner.topology.peers();
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
    pub async fn promote_node(&self, node: &NodeId) {
        let mut inner = self.lock.write().await;
        inner.topology.promote_peer(node.as_ref());
    }

    /// register a strike against the given peer
    pub async fn report_node(&self, node_id: &NodeId) {
        let mut inner = self.lock.write().await;
        if let Some(node) = inner.topology.get(node_id.as_ref()).cloned() {
            if inner.quarantine.quarantine_node((&node).into()) {
                inner.topology.remove_peer(node_id.as_ref());
                // Trusted peers in poldercast requires to be demoted 2 times before
                // moving to the dirty pool
                inner.topology.remove_peer(node_id.as_ref());
            }
        }
    }
}
