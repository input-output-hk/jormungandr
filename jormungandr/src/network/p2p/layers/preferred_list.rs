pub use jormungandr_lib::interfaces::{PreferredListConfig, TrustedPeer};
use poldercast::{Address, GossipsBuilder, Layer, NodeProfile, Nodes, ViewBuilder};
use rand::seq::IteratorRandom;
use rand::{Rng as _, SeedableRng};
use rand_chacha::ChaChaRng;
use std::collections::HashSet;

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
    pub fn new(config: PreferredListConfig) -> Self {
        let mut seed = [0; 32];

        rand::thread_rng().fill(&mut seed);
        let addresses: Vec<Address> = config.peers.iter().map(|p| p.address.clone()).collect();
        Self::new_with_seed(config.view_max.into(), addresses, seed)
    }

    fn new_with_seed(
        view_max: usize,
        peers: Vec<Address>,
        seed: <ChaChaRng as SeedableRng>::Seed,
    ) -> Self {
        Self {
            view_max,
            peers: peers.iter().cloned().collect(),
            prng: ChaChaRng::from_seed(seed),
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
