//! Multiverse
//!
//! This is a multi temporal store, where the timeline is accessible by HeaderHash
//! and multiple timelines are possible.
//!
//! For now this only track block at the headerhash level, and doesn't order them
//! temporaly, leaving no way to do garbage collection

use crate::block::ChainLength;
use crate::ledger::Ledger;
use chain_core::property::{BlockId as _, HasMessages as _};
use chain_storage::store::BlockStore;
use std::collections::{hash_map::Entry, BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

type BlockId = crate::key::Hash;

//type StateLedger = crate::ledger::Ledger;

//
// The multiverse is characterized by a single origin and multiple state of a given time
//
//          [root A]
//        ,o            ,-o-o--o [root B]
//       /             /
// o----o----o--o--o--o-o-o-o-oooo [root E]
//                  \
//                   `-o--o [root C]
//                      \
//                      `----o-o-oo [root F]
//
// +------------------------------+-----> time
// t=0                            t=latest known
//
pub struct Multiverse<State> {
    states_by_hash: HashMap<BlockId, State>,
    states_by_chain_length: BTreeMap<ChainLength, HashSet<BlockId>>, // FIXME: use multimap?
    roots: Arc<RwLock<Roots>>,
}

/// Keep all states that are this close to the longest chain.
const SUFFIX_TO_KEEP: u32 = 50;

struct Roots {
    /// Record how many GCRoot objects currently exist for this block ID.
    roots: HashMap<BlockId, usize>,
}

/// A RAII wrapper around a block identifier that keeps the state
/// corresponding to the block pinned in memory.
pub struct GCRoot {
    hash: BlockId,
    roots: Arc<RwLock<Roots>>,
}

impl GCRoot {
    fn new(hash: BlockId, roots: Arc<RwLock<Roots>>) -> Self {
        {
            let mut roots = roots.write().unwrap();
            *roots.roots.entry(hash.clone()).or_insert(0) += 1;
        }

        GCRoot { hash, roots }
    }
}

impl std::ops::Deref for GCRoot {
    type Target = BlockId;
    fn deref(&self) -> &Self::Target {
        &self.hash
    }
}

impl Drop for GCRoot {
    fn drop(&mut self) {
        let mut roots = self.roots.write().unwrap();
        if let Entry::Occupied(mut entry) = roots.roots.entry(self.hash.clone()) {
            if *entry.get() > 1 {
                *entry.get_mut() -= 1;
            } else {
                //println!("state for block {:?} became garbage", self.hash);
                entry.remove_entry();
                // put on GC list?
            }
        } else {
            unreachable!();
        }
    }
}

impl<State> Multiverse<State> {
    pub fn new() -> Self {
        Multiverse {
            states_by_hash: HashMap::new(),
            states_by_chain_length: BTreeMap::new(),
            roots: Arc::new(RwLock::new(Roots {
                roots: HashMap::new(),
            })),
        }
    }
    fn make_root(&mut self, k: BlockId) -> GCRoot {
        debug_assert!(self.states_by_hash.contains_key(&k));
        GCRoot::new(k, self.roots.clone())
    }

    pub fn get(&self, k: &BlockId) -> Option<&State> {
        self.states_by_hash.get(&k)
    }

    pub fn get_from_root(&self, root: &GCRoot) -> &State {
        assert!(Arc::ptr_eq(&root.roots, &self.roots));
        self.get(&*root).unwrap()
    }
}

impl Multiverse<Ledger> {
    /// Add a state to the multiverse. Return a GCRoot object that
    /// pins the state into memory.
    pub fn add(&mut self, k: BlockId, st: Ledger) -> GCRoot {
        self.states_by_chain_length
            .entry(st.chain_length())
            .or_insert(HashSet::new())
            .insert(k.clone());
        self.states_by_hash.entry(k.clone()).or_insert(st);
        self.make_root(k)
    }

    fn delete(&mut self, k: &BlockId) {
        //println!("deleting state {:?}", k);
        let st = self.states_by_hash.remove(&k).unwrap();
        // Remove the hash from states_by_chain_length, then prune
        // the latter.
        if let std::collections::btree_map::Entry::Occupied(mut entry) =
            self.states_by_chain_length.entry(st.chain_length())
        {
            let removed = entry.get_mut().remove(&k);
            assert!(removed);
            if entry.get().is_empty() {
                //println!("removing chain length {}", st.chain_length().0);
                entry.remove_entry();
            }
        } else {
            unreachable!();
        }
    }

    /// Once the state are old in the timeline, they are less
    /// and less likely to be used anymore, so we leave
    /// a gap between different version that gets bigger and bigger
    pub fn gc(&mut self) {
        let mut garbage = vec![];

        {
            let roots = self.roots.read().unwrap();

            let longest_chain = self.states_by_chain_length.iter().next_back().unwrap().0;

            let mut to_keep = ChainLength(0);

            for (chain_length, hashes) in &self.states_by_chain_length {
                // Keep states close to the current longest
                // chain. FIXME: we should keep only the state that is
                // an ancestor of the current longest chain. However,
                // checking ancestry requires access to BlockStore.
                if chain_length.0 + SUFFIX_TO_KEEP >= longest_chain.0 {
                    break;
                }
                // Keep states in gaps that get exponentially smaller
                // as they get closer to the longest chain.
                if chain_length >= &to_keep {
                    to_keep = ChainLength(chain_length.0 + (longest_chain.0 - chain_length.0) / 2);
                } else {
                    for k in hashes {
                        // Keep states that are GC roots.
                        if !roots.roots.contains_key(&k) {
                            garbage.push(k.clone());
                        }
                    }
                }
            }
        }

        //println!("deleting {} states from multiverse", garbage.len());

        for k in garbage {
            self.delete(&k);
        }
    }

    /// Get the chain state at block 'k' from memory if present;
    /// otherwise reconstruct it by reading blocks from storage and
    /// applying them to the nearest ancestor state that we do have.
    pub fn get_from_storage<S: BlockStore<Block = crate::block::Block>>(
        &mut self,
        k: BlockId,
        store: &S,
    ) -> Result<GCRoot, chain_storage::error::Error> {
        if let Some(_) = self.states_by_hash.get(&k) {
            return Ok(self.make_root(k));
        }

        // Find the most recent ancestor that we have in
        // memory. FIXME: could do a binary search here on the chain
        // length interval between 0 and k.chain_length(), though it
        // doesn't matter much for complexity since we need to apply
        // O(n) blocks anyway.

        let mut blocks_to_apply = vec![];
        let mut cur_hash = k.clone();

        let mut state = loop {
            if cur_hash == BlockId::zero() {
                panic!("don't know how to reconstruct initial chain state");
            }

            if let Some(state) = self.get(&cur_hash) {
                break state.clone();
            }

            let cur_block_info = store.get_block_info(&cur_hash).unwrap();
            blocks_to_apply.push(k.clone());
            cur_hash = cur_block_info.parent_id();
        };

        /*
        println!(
            "applying {} blocks to reconstruct state",
            blocks_to_apply.len()
        );
        */

        for hash in blocks_to_apply.iter().rev() {
            let block = store.get_block(&hash).unwrap().0;
            state = state
                .apply_block(&state.get_ledger_parameters(), block.messages())
                .unwrap();
            // FIXME: add the intermediate states to memory?
        }

        Ok(self.add(k, state))
    }

    /// Return the number of states stored in memory.
    pub fn nr_states(&self) -> usize {
        self.states_by_hash.len()
    }
}

#[cfg(test)]
mod test {

    use super::Multiverse;
    use crate::block::{Block, BlockBuilder};
    use crate::ledger::Ledger;
    use crate::message::{InitialEnts, Message};
    use chain_core::property::{Block as _, ChainLength as _, HasMessages as _};
    use chain_storage::store::BlockStore;
    use quickcheck::StdGen;

    fn apply_block(state: &Ledger, block: &Block) -> Ledger {
        if state.chain_length().0 != 0 {
            assert_eq!(state.chain_length().0 + 1, block.chain_length().0);
        }
        state
            .apply_block(&state.get_ledger_parameters(), block.messages())
            .unwrap()
    }

    #[test]
    pub fn multiverse() {
        let mut multiverse = Multiverse::new();

        let mut g = StdGen::new(rand::thread_rng(), 10);
        let leader_key = crate::key::test::arbitrary_secret_key(&mut g);

        let mut store = chain_storage::memory::MemoryBlockStore::new();

        let mut genesis_block = BlockBuilder::new();
        genesis_block.message(Message::Initial(InitialEnts::new()));
        let genesis_block = genesis_block.make_genesis_block();
        let genesis_state = Ledger::new(genesis_block.id(), genesis_block.messages()).unwrap();
        assert_eq!(genesis_state.chain_length().0, 0);
        store.put_block(&genesis_block).unwrap();
        multiverse.add(genesis_block.id(), genesis_state.clone());

        let mut state = genesis_state;
        let mut _root = None;
        let mut parent = genesis_block.id();
        let mut ids = vec![];
        for i in 1..10001 {
            let mut block = BlockBuilder::new();
            block.chain_length(state.chain_length.next());
            block.parent(parent);
            let block = block.make_bft_block(&leader_key);
            state = apply_block(&state, &block);
            assert_eq!(state.chain_length().0, i);
            store.put_block(&block).unwrap();
            _root = Some(multiverse.add(block.id(), state.clone()));
            multiverse.gc();
            ids.push(block.id());
            parent = block.id();
            assert!(
                multiverse.nr_states()
                    <= super::SUFFIX_TO_KEEP as usize + ((i as f32).log2()) as usize
            );
        }

        {
            let root = multiverse
                .get_from_storage(ids[9999].clone(), &store)
                .unwrap();
            let state = multiverse.get_from_root(&root);
            assert_eq!(state.chain_length().0, 10000);
        }

        {
            let root = multiverse
                .get_from_storage(ids[1234].clone(), &store)
                .unwrap();
            let state = multiverse.get_from_root(&root);
            assert_eq!(state.chain_length().0, 1235);
        }

        {
            let root = multiverse
                .get_from_storage(ids[9500].clone(), &store)
                .unwrap();
            let state = multiverse.get_from_root(&root);
            assert_eq!(state.chain_length().0, 9501);
        }

        let before = multiverse.nr_states();
        multiverse.gc();
        let after = multiverse.nr_states();
        assert_eq!(before, after + 2);
    }

}
