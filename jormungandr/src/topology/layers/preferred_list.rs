use crate::{settings::start::network::TrustedPeer, topology::NodeId};
use poldercast::{
    layer::{Layer, ViewBuilder},
    InterestLevel, PriorityMap, Profile, Topic,
};
use rand::seq::IteratorRandom;
use rand_chacha::ChaChaRng;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
};

#[derive(Clone)]
pub struct PreferredListConfig {
    pub view_max: usize,
    pub peers: Vec<TrustedPeer>,
}

/// This layer always return a view containing only a subset
/// of the preferred peers that are known to the topology
pub struct PreferredListLayer {
    /// the max number of entries to add in the list of the view
    view_max: usize,
    /// the preferred peers list
    peers: HashMap<SocketAddr, Option<NodeId>>,
    /// actual peers that are known to the topology
    current_peers: HashSet<keynesis::key::ed25519::PublicKey>,
    /// a pseudo random number generator, this will help with
    /// testing and reproducing issues.
    ///
    /// Do not let a user seed the value, while having a secure
    /// RNG is not necessary it is functionally important to allow
    /// for randomness to make its course.
    prng: rand_chacha::ChaChaRng,
}

impl PreferredListLayer {
    pub fn new(config: &PreferredListConfig, prng: ChaChaRng) -> Self {
        Self {
            view_max: config.view_max,
            peers: config
                .peers
                .iter()
                .map(|peer| (peer.addr, peer.id))
                .collect(),
            current_peers: HashSet::new(),
            prng,
        }
    }
}

impl Layer for PreferredListLayer {
    fn name(&self) -> &'static str {
        "custom::preferred_list"
    }

    fn view(&mut self, builder: &mut ViewBuilder) {
        self.current_peers
            .iter()
            .choose_multiple(&mut self.prng, self.view_max)
            .into_iter()
            .for_each(|id| builder.add(id));
    }

    // Preferred nodes will never be quarantined
    fn remove(&mut self, _: &keynesis::key::ed25519::PublicKey) {}

    fn reset(&mut self) {
        self.current_peers.clear();
    }

    fn subscribe(&mut self, _: Topic) {}

    fn unsubscribe(&mut self, _: &Topic) {}

    fn subscriptions(&self, _: &mut PriorityMap<InterestLevel, Topic>) {}

    fn populate(&mut self, _: &Profile, new_profile: &Profile) {
        let addr = new_profile.address();
        let id = new_profile.id();
        if let Some(trusted_id) = self.peers.get(&addr) {
            if trusted_id.is_none() || trusted_id.as_ref().map(AsRef::as_ref) == Some(&id) {
                self.current_peers.insert(id);
            }
        }
    }
}
