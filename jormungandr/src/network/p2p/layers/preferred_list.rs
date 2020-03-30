pub use poldercast::Address;
use poldercast::{GossipsBuilder, Layer, NodeProfile, Nodes, ViewBuilder};
use rand::{seq::SliceRandom as _, Rng as _, SeedableRng};
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};

const DEFAULT_PREFERRED_VIEW_MAX: usize = 20;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PreferredListConfig {
    #[serde(default)]
    view_max: PreferredViewMax,

    #[serde(default)]
    peers: Vec<Address>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct PreferredViewMax(usize);

pub struct PreferredListLayer {
    /// the max number of entries to add in the list of the view
    view_max: usize,

    /// the buddy list
    peers: Vec<Address>,

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

        Self::new_with_seed(config.view_max.into(), config.peers, seed)
    }

    pub fn new_with_seed(
        view_max: usize,
        peers: Vec<Address>,
        seed: <ChaChaRng as SeedableRng>::Seed,
    ) -> Self {
        Self {
            view_max,
            peers,
            prng: ChaChaRng::from_seed(seed),
        }
    }
}

impl Layer for PreferredListLayer {
    fn alias(&self) -> &'static str {
        "custom::preferred_list"
    }

    fn reset(&mut self) {}

    fn populate(&mut self, identity: &NodeProfile, all_nodes: &Nodes) {}

    fn gossips(&mut self, identity: &NodeProfile, gossips: &mut GossipsBuilder, all_nodes: &Nodes) {
    }

    fn view(&mut self, view: &mut ViewBuilder, all_nodes: &mut Nodes) {
        let selected = self.peers.choose_multiple(&mut self.prng, self.view_max);

        for selected in selected {
            let node = all_nodes.peek(selected);
            view.add(node);
        }
    }
}

impl Default for PreferredViewMax {
    fn default() -> Self {
        Self(DEFAULT_PREFERRED_VIEW_MAX)
    }
}

impl From<PreferredViewMax> for usize {
    fn from(pvm: PreferredViewMax) -> Self {
        pvm.0
    }
}
