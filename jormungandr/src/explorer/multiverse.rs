use crate::blockcfg::{ChainLength, HeaderHash, Multiverse as MultiverseData};
use chain_impl_mockchain::multiverse;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::State;

pub struct Multiverse {
    inner: Arc<RwLock<Inner>>,
}

pub(super) type Ref = multiverse::Ref<State>;

struct Inner {
    multiverse: MultiverseData<State>,
    tips: HashSet<HeaderHash>,
}

impl Multiverse {
    pub(super) fn new(
        chain_length: ChainLength,
        block0_id: HeaderHash,
        initial_state: State,
    ) -> (multiverse::Ref<State>, Self) {
        let mut multiverse = MultiverseData::new();
        let initial_ref = multiverse.insert(chain_length, block0_id, initial_state);

        let mut tips = HashSet::new();
        tips.insert(block0_id);

        (
            initial_ref,
            Multiverse {
                inner: Arc::new(RwLock::new(Inner { multiverse, tips })),
            },
        )
    }

    pub(super) async fn insert(
        &self,
        chain_length: ChainLength,
        parent: HeaderHash,
        hash: HeaderHash,
        value: State,
    ) -> multiverse::Ref<State> {
        let mut guard = self.inner.write().await;

        guard.tips.remove(&parent);
        guard.tips.insert(hash);
        guard.multiverse.insert(chain_length, hash, value)
    }

    pub(super) async fn get_ref(&self, hash: HeaderHash) -> Option<multiverse::Ref<State>> {
        let guard = self.inner.read().await;
        guard.multiverse.get_ref(&hash)
    }

    pub(super) async fn get(&self, hash: HeaderHash) -> Option<State> {
        let guard = self.inner.read().await;
        guard.multiverse.get(&hash).as_deref().cloned()
    }

    /// run the garbage collection of the multiverse
    ///
    pub async fn gc(&self, depth: u32) {
        let mut guard = self.inner.write().await;
        guard.multiverse.gc(depth)
    }

    /// get all the branches this block is in, None here means the block was never added
    /// or it was moved to stable storage
    pub(super) async fn tips(&self) -> Vec<Arc<State>> {
        let mut guard = self.inner.write().await;
        let mut states = Vec::new();

        // garbage collect old tips too
        let mut new_tips = HashSet::new();

        for tip in guard.tips.iter() {
            if let Some(state) = guard.multiverse.get(&tip) {
                // TODO: probably return them sorted by chain length (descending)?
                states.push(state);

                new_tips.insert(*tip);
            }
        }

        guard.tips = new_tips;

        states
    }
}

impl Clone for Multiverse {
    fn clone(&self) -> Self {
        Multiverse {
            inner: self.inner.clone(),
        }
    }
}
