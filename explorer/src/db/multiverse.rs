use super::State;
use chain_impl_mockchain::{
    block::{ChainLength, HeaderId as HeaderHash},
    multiverse,
};
use multiverse::Multiverse as MultiverseData;
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::RwLock;

pub struct Multiverse {
    inner: Arc<RwLock<Inner>>,
}

pub type Ref = multiverse::Ref<State>;

struct Inner {
    multiverse: MultiverseData<State>,
    tips: BTreeSet<(ChainLength, HeaderHash)>,
}

impl Multiverse {
    pub(super) fn new(
        chain_length: ChainLength,
        block0_id: HeaderHash,
        initial_state: State,
    ) -> (multiverse::Ref<State>, Self) {
        let mut multiverse = MultiverseData::new();
        let initial_ref = multiverse.insert(chain_length, block0_id, initial_state);

        let mut tips = BTreeSet::new();
        tips.insert((chain_length, block0_id));

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

        guard
            .tips
            .remove(&(chain_length.nth_ancestor(1).unwrap(), parent));
        guard.tips.insert((chain_length, hash));
        guard.multiverse.insert(chain_length, hash, value)
    }

    pub(super) async fn get_ref(&self, hash: &HeaderHash) -> Option<multiverse::Ref<State>> {
        let guard = self.inner.read().await;
        guard.multiverse.get_ref(hash)
    }

    /// run the garbage collection of the multiverse
    ///
    pub(super) async fn gc(&self, depth: u32) {
        let mut guard = self.inner.write().await;
        guard.multiverse.gc(depth)
    }

    /// get all the branches this block is in, None here means the block was never added
    /// or it was moved to stable storage
    pub(super) async fn tips(&self) -> Vec<(HeaderHash, multiverse::Ref<State>)> {
        let mut guard = self.inner.write().await;
        let mut states = Vec::new();

        // garbage collect old tips too
        let mut new_tips = BTreeSet::new();

        for (length, hash) in guard.tips.iter().rev() {
            if let Some(state) = guard.multiverse.get_ref(hash) {
                states.push((*hash, state));
                new_tips.insert((*length, *hash));
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
