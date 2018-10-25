use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Effectively a strand of the blockchain, grafted to a parent hash,
/// and where the tip is tracked and all the block component.
///
/// Use this for effective tracking of different forks, and allow
/// to clean quickly an abandonned fork.
pub struct Strand<Hash, Block> {
    tip: Hash,
    parent: Hash,
    blocks: BTreeMap<Hash, Block>,
}

impl<Hash: Ord+Clone, Block> Strand<Hash, Block> {
    /// Create a new strand
    pub fn new(hash: Hash, parent: Hash, block: Block) -> Self {
        let mut blocks = BTreeMap::new();
        blocks.insert(hash.clone(), block);
        Strand { tip : hash, parent: parent, blocks: blocks }
    }

    /// Extend the strand without any checks
    pub fn extend(&mut self, hash: Hash, block: Block) {
        self.blocks.insert(hash.clone(), block);
        self.tip = hash;
    }
}

/// Track all the known tips
pub struct ChainTips<Hash> {
    tips: BTreeSet<Hash>
}

impl<Hash: Ord> ChainTips<Hash> {
    pub fn new() -> Self {
        ChainTips { tips: BTreeSet::new() }
    }

    /// try to move a tip, if the tip doesn't exist, then it create a new one
    pub fn move_tip(&mut self, ancient: Hash, new: Hash) {
        let _ = self.tips.remove(&ancient);
        self.tips.insert(new);
    }
}
