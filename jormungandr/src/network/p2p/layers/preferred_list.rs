pub use jormungandr_lib::interfaces::{PreferredListConfig, TrustedPeer};
use poldercast::layer::Layer;
use rand::seq::IteratorRandom;
use rand_chacha::ChaChaRng;
use std::collections::HashSet;

struct Address;

pub struct PreferredListLayer {
    /// the max number of entries to add in the list of the view
    view_max: usize,

    /// the buddy list
    peers: HashSet<Address>,

    /// a pseudo random number generator, this will help with
    /// testing and reproducing issues.
    ///
    /// Do not let a user seed the value, while having a secure
    /// RNG is not necessary it is functionally important to allow
    /// for randomness to make its course.
    prng: rand_chacha::ChaChaRng,
}

impl PreferredListLayer {
    pub fn new(config: PreferredListConfig, prng: ChaChaRng) -> Self {
        let addresses: Vec<Address> = config.peers.iter().map(|p| p.address.clone()).collect();
        Self {
            view_max: config.view_max.into(),
            peers: addresses.into_iter().collect(),
            prng,
        }
    }
}

impl Layer for PreferredListLayer {
    fn alias(&self) -> &'static str {
        "custom::preferred_list"
    }

    fn reset(&mut self) {}

    fn populate(&mut self, _identity: &NodeProfile, _all_nodes: &Nodes) {}

    fn gossips(
        &mut self,
        _identity: &NodeProfile,
        _gossips: &mut GossipsBuilder,
        _all_nodes: &Nodes,
    ) {
    }

    fn view(&mut self, view: &mut ViewBuilder, _all_nodes: &mut Nodes) {
        self.peers
            .iter()
            .choose_multiple(&mut self.prng, self.view_max)
            .into_iter()
            .for_each(|address| view.add_address(address.clone()));
    }
}
