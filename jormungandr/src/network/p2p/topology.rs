//! module defining the p2p topology management objects
//!

use crate::{
    network::p2p::{Address, Gossip, Gossips, Quarantine},
    settings::start::network::Configuration,
};
use jormungandr_lib::time::SystemTime;
use poldercast::{Profile, Topology};
use tokio::sync::RwLock;
use tracing::instrument;

use super::{topic, NodeId};

use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

lazy_static! {
    static ref LOCAL_ADDR: Address = Address::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
}

#[derive(Eq, Clone, Serialize, Debug)]
pub struct ProfileInfo {
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub id: NodeId,
    pub address: Address,
    pub last_update: SystemTime,
    pub quarantined: Option<SystemTime>,
    pub subscriptions: Vec<(String, String)>,
}

impl PartialEq for ProfileInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.address == other.address
    }
}

impl Hash for ProfileInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.address.hash(state);
    }
}

impl From<&Arc<Profile>> for ProfileInfo {
    fn from(other: &Arc<Profile>) -> Self {
        Self {
            id: other.id(),
            address: other.address(),
            last_update: other.last_update().to_system_time().into(),
            quarantined: None,
            subscriptions: other
                .subscriptions()
                .iter()
                .map(|s| (s.topic().to_string(), format!("{:?}", s.interest_level())))
                .collect(),
        }
    }
}

pub struct View {
    pub peers: Vec<Arc<Profile>>,
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
        let addr = config.public_address.or(Some(*LOCAL_ADDR)).unwrap();
        let key = super::secret_key_into_keynesis(config.node_key.clone());

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
        let peers = inner.topology.view(None, selection).into_iter().collect();
        View { peers }
    }

    // If the recipient is not specified gossip will only contain information
    // about this node
    pub async fn initiate_gossips(&self, recipient: Option<&NodeId>) -> Gossips {
        let mut inner = self.lock.write().await;
        let mut gossips = if let Some(recipient) = recipient {
            inner.topology.gossips_for(recipient)
        } else {
            Vec::new()
        };
        // If the recipient is not already in the topology
        // or was not specified poldercast will not return anything.
        // Let's broadcast out profile anyway
        if gossips.is_empty() {
            gossips.push(inner.topology.self_profile().gossip().clone());
        }
        gossips.retain(|g| g.address() != *LOCAL_ADDR);
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
            let is_dirty = inner.topology.peers().dirty().contains(&node.id);
            if is_dirty {
                tracing::debug!(node = %node.address, "lifting node from quarantine");
                inner.topology.promote_peer(&node.id);
            } else {
                tracing::debug!(node = %node.address, "node from quarantine have left the dirty pool. skipping it");
            }
        }
    }

    // This may return nodes that are still quarantined but have been
    // forgotten by the underlying poldercast implementation.
    pub async fn list_quarantined(&self) -> Vec<ProfileInfo> {
        self.lock.read().await.quarantine.quarantined_nodes()
    }

    pub async fn list_available(&self) -> Vec<ProfileInfo> {
        let inner = self.lock.read().await;
        let profiles = inner.topology.peers();
        profiles
            .pool()
            .iter()
            .chain(profiles.trusted().iter())
            .map(|(_, profile)| profile.into())
            .collect()
    }

    pub async fn list_non_public(&self) -> Vec<ProfileInfo> {
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
        inner.topology.promote_peer(node);
    }

    /// register a strike against the given peer
    pub async fn report_node(&self, node_id: &NodeId) {
        let mut inner = self.lock.write().await;
        if let Some(node) = inner.topology.get(node_id).cloned() {
            if inner.quarantine.quarantine_node((&node).into()) {
                inner.topology.remove_peer(node_id);
                // Don't know what is the purpose of trusted peers in poldercast,
                // this is a quick hack to treat those as standard ones
                inner.topology.remove_peer(node_id);
            }
        }
    }
}
