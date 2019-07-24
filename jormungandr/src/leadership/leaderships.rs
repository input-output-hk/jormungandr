use crate::blockcfg::{ChainLength, Epoch, Header, HeaderHash};
use chain_core::property::Header as _;
use chain_impl_mockchain::multiverse::{GCRoot, Multiverse};
use std::collections::{BTreeMap, HashSet};

pub use chain_impl_mockchain::leadership::Leadership;

/// structure containing the leaderships at different
/// time of the blockchain.
pub struct Leaderships {
    multiverse: Multiverse<Leadership>,

    anchors: BTreeMap<Epoch, HashSet<HeaderHash>>,
}

impl Leaderships {
    pub fn new() -> Self {
        Leaderships {
            multiverse: Multiverse::new(),
            anchors: BTreeMap::new(),
        }
    }

    pub fn get(&self, epoch: Epoch) -> Option<impl Iterator<Item = (&HeaderHash, &Leadership)>> {
        self.anchors.get(&epoch).map(|set| {
            set.iter()
                .map(move |h| (h, self.multiverse.get(h).unwrap()))
        })
    }

    pub fn add(
        &mut self,
        epoch: Epoch,
        chain_length: ChainLength,
        header_hash: HeaderHash,
        leadership: Leadership,
    ) -> GCRoot {
        let gc_root = self
            .multiverse
            .insert(chain_length, header_hash, leadership);

        self.anchors
            .entry(epoch)
            .or_insert(HashSet::new())
            .insert(*gc_root);
        gc_root
    }
}
