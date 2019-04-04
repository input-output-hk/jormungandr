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

    pub tip: GCRoot,
}

impl Leaderships {
    /// Leadership object need to be construction with initial data
    pub fn new(block_0_header: &Header, initial: Leadership) -> Self {
        let mut multiverse = Multiverse::new();
        let mut anchors = BTreeMap::new();

        let gc_root =
            multiverse.insert(block_0_header.chain_length(), block_0_header.id(), initial);

        anchors
            .entry(block_0_header.date().epoch)
            .or_insert(HashSet::new())
            .insert(*gc_root);

        Leaderships {
            multiverse: multiverse,
            anchors: anchors,
            tip: gc_root,
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
