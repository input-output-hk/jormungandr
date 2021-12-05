use crate::{Error, Reference, Selection};
use chain_impl_mockchain::{block::Block, header::HeaderId, ledger::RewardsInfoParameters};
use std::sync::Arc;

pub struct Blockchain {
    tip: Arc<Reference>,
    heads: lru::LruCache<HeaderId, Arc<Reference>>,
    cache: lru::LruCache<HeaderId, Arc<Reference>>,
}

/// configuration entries for the blockchain parameters
#[derive(Debug, Copy, Clone)]
pub struct Configuration {
    pub heads_capacity: usize,
    pub cache_capacity: usize,
    pub rewards_info_params: RewardsInfoParameters,
}

pub enum Event {
    Added {
        new_branch: bool,
        new_tip: bool,
        epoch_transition: bool,
        new_reference: Arc<Reference>,
    },
    MissingParent {
        parent: HeaderId,
    },
}

impl Blockchain {
    /// start a blockchain with the given block as starting point
    pub fn new(configuration: &Configuration, block0: Arc<Reference>) -> Self {
        let mut blockchain = Self {
            tip: Arc::clone(&block0),
            heads: lru::LruCache::new(configuration.heads_capacity),
            cache: lru::LruCache::new(configuration.cache_capacity),
        };

        blockchain.heads.put(block0.hash(), Arc::clone(&block0));
        blockchain.cache.put(block0.hash(), block0);

        blockchain
    }

    pub fn tip(&self) -> Arc<Reference> {
        Arc::clone(&self.tip)
    }

    /// get an iterator for all the branches currently being considered by
    /// the `Blockchain`.
    ///
    /// The `tip` is already included in the list too and it may be that
    /// the some branches in the list are no longer `Head` only.
    pub fn branches(&self) -> lru::Iter<'_, HeaderId, Arc<Reference>> {
        self.heads.iter()
    }

    pub fn put(&mut self, block: &Block) -> Result<Event, Error> {
        let parent_hash = block.header().block_parent_hash();
        if let Some(parent) = self.heads.get(&parent_hash).cloned() {
            // refresh the parent in the `cache` LRU
            self.cache.put(parent_hash, Arc::clone(&parent));
            let new_reference = Reference::chain(Arc::clone(&parent), block)?;
            let new_reference = Arc::new(new_reference);
            let epoch_transition = parent.block_date().epoch < new_reference.block_date().epoch;

            Ok(self.put_head(new_reference, false, epoch_transition))
        } else if let Some(parent) = self.cache.get(&parent_hash).cloned() {
            let new_reference = Reference::chain(Arc::clone(&parent), block)?;
            let new_reference = Arc::new(new_reference);
            let epoch_transition = parent.block_date().epoch < new_reference.block_date().epoch;

            Ok(self.put_head(new_reference, true, epoch_transition))
        } else {
            Ok(Event::MissingParent {
                parent: block.header().hash(),
            })
        }
    }

    fn put_head(
        &mut self,
        reference: Arc<Reference>,
        new_branch: bool,
        epoch_transition: bool,
    ) -> Event {
        self.heads.put(reference.hash(), Arc::clone(&reference));
        self.cache.put(reference.hash(), Arc::clone(&reference));

        let new_tip = match self.tip.select(&reference) {
            Selection::PreferCurrent => {
                // we prefer the current tip, so refresh it in the cache so
                // it is properly cached and we don't get cache miss in
                // future `put`.
                self.heads.put(self.tip.hash(), Arc::clone(&self.tip));
                self.cache.put(self.tip.hash(), Arc::clone(&self.tip));
                false
            }
            Selection::PreferCandidate => {
                self.tip = Arc::clone(&reference);
                true
            }
        };

        Event::Added {
            new_tip,
            new_branch,
            epoch_transition,
            new_reference: reference,
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            heads_capacity: 1024,
            cache_capacity: 1024 * 1024 * 1024,
            rewards_info_params: RewardsInfoParameters::default(),
        }
    }
}
