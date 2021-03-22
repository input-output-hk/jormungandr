//! module defining the p2p topology management objects
//!

use crate::{
    network::p2p::{Address, Gossip, Gossips, Quarantine},
    settings::start::network::Configuration,
};
use jormungandr_lib::time::SystemTime;
use poldercast::{Profile, Subscription, Subscriptions, Topology};
use serde::Serializer;
use tokio::sync::RwLock;
use tracing::instrument;

use super::{topic, NodeId};

use std::hash::{Hash, Hasher};
use std::sync::Arc;

fn serialize_display<T: std::fmt::Display, S>(item: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&item.to_string())
}

#[derive(Eq, Clone, Serialize, Debug)]
pub struct ProfileInfo {
    #[serde(serialize_with = "serialize_display")]
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

impl From<Arc<Profile>> for ProfileInfo {
    fn from(other: Arc<Profile>) -> Self {
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
    pub self_node: Arc<Profile>,
    pub peers: Vec<Arc<Profile>>,
}

struct Inner {
    topology: Topology,
    quarantine: Quarantine,
    // this is needed to advertise ourself to trusted peers,
    // poldercast does not allow us to take this out from the
    // topology
    initial_self_profile: Arc<Profile>,
}

/// object holding the P2pTopology of the Node
pub struct P2pTopology {
    lock: RwLock<Inner>,
}

impl P2pTopology {
    pub fn new(config: &Configuration) -> Self {
        // This is needed at the beginning to advert ourself to trusted peers
        let subscriptions = crate::settings::start::config::default_interests()
            .into_iter()
            .fold(Subscriptions::new(), |mut acc, (topic, interest)| {
                acc.push(Subscription::new(topic.0, interest.0).as_slice())
                    .unwrap();
                acc
            });

        // FIXME: How should we handle cases where the is not listen set? Can a node just receive?
        let addr = config.public_address.unwrap();
        let key = super::secret_key_into_keynesis(config.node_key.clone());
        let initial_self_profile = Arc::new(Profile::from(poldercast::Gossip::new(
            addr,
            &key,
            subscriptions.as_slice(),
        )));

        let quarantine = Quarantine::from_config(config.policy.clone());
        let mut topology = Topology::new(addr, &key);
        topology.subscribe_topic(topic::MESSAGES);
        topology.subscribe_topic(topic::BLOCKS);
        let inner = Inner {
            topology,
            initial_self_profile,
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
        View {
            self_node: inner.initial_self_profile.clone(),
            peers,
        }
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
            gossips.push(inner.initial_self_profile.gossip().clone());
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
            inner.topology.promote_peer(&node.id);
        }
    }

    #[instrument(skip(self), level = "debug")]
    pub async fn force_reset_layers(&self) {
        tracing::warn!("resetting layers is not supported in this poldercast version");
    }

    // FIXME: Poldercast is lacking the abitily to return all the nodes currently
    // in the topology, without that it would be very inefficient to track
    // nodes due to transparent eviction from the underlying lru cache.
    // Until that is implemented this method may return nodes that have been
    // remove completely from the topology
    pub async fn list_quarantined(&self) -> Vec<ProfileInfo> {
        self.lock.read().await.quarantine.quarantined_nodes()
    }

    // FIXME: Poldercast is lacking the abitily to return all the nodes currently
    // in the topology, without that it would be very inefficient to track
    // nodes due to transparent eviction from the underlying lru cache.
    // Until that is implemented this method may not reflect all available peers
    pub async fn list_available(&self) -> Vec<ProfileInfo> {
        self.view(poldercast::layer::Selection::Any)
            .await
            .peers
            .into_iter()
            .map(|profile| profile.into())
            .collect()
    }

    // FIXME: Poldercast is lacking the abitily to return all the nodes currently
    // in the topology, without that it would be very inefficient to track
    // nodes due to transparent eviction from the underlying lru cache.
    // Until that is implemented this method may not reflect all available peers
    pub async fn list_non_public(&self) -> Vec<ProfileInfo> {
        self.view(poldercast::layer::Selection::Any)
            .await
            .peers
            .into_iter()
            .filter_map(|profile| {
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
            if inner.quarantine.quarantine_node(node.into()) {
                inner.topology.remove_peer(node_id);
                // Don't know what is the purpose of trusted peers in poldercast,
                // this is a quick hack to treat those as standard ones
                inner.topology.remove_peer(node_id);
            }
        }
    }
}
