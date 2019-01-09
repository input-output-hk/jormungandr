use chain_core::property::{Block, ChainState, ChainStateDelta, Serializable};
use super::store::{BlockInfo, BlockStore, ChainStateStore};
use super::error::Error;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct MemoryBlockStore<C> where C: ChainState {
    genesis_hash: <C::Block as Block>::Id,
    genesis_chain_state: C,
    blocks: HashMap<<C::Block as Block>::Id, (Vec<u8>, BlockInfo<<C::Block as Block>::Id, <C::Block as Block>::Date>)>,
    chain_state_deltas: HashMap<<C::Block as Block>::Id, (<C::Block as Block>::Id, Vec<u8>)>,
    tags: HashMap<String, <C::Block as Block>::Id>,
}

impl<C> MemoryBlockStore<C> where C: ChainState {
    pub fn new(genesis_data: &C::GenesisData) -> Self {
        let genesis_chain_state = C::new(genesis_data).unwrap();
        MemoryBlockStore {
            genesis_hash: genesis_chain_state.get_last_block_id(),
            genesis_chain_state,
            blocks: HashMap::new(),
            chain_state_deltas: HashMap::new(),
            tags: HashMap::new(),
        }
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

impl<C> Drop for MemoryBlockStore<C> where C: ChainState {
    fn drop(&mut self) {
        println!("number of block info reads = {}", COUNTER.load(Ordering::SeqCst));
    }
}

impl<C> BlockStore<C::Block> for MemoryBlockStore<C> where C: ChainState {

    fn put_block_internal(&mut self, block: C::Block, block_info: BlockInfo<<C::Block as Block>::Id, <C::Block as Block>::Date>) -> Result<(), Error>
    {
        self.blocks.insert(
            block_info.block_hash.clone(),
            (block.serialize_as_vec().unwrap(), block_info));
        Ok(())
    }

    fn get_block(&self, block_hash: &<C::Block as Block>::Id) -> Result<(C::Block, BlockInfo<<C::Block as Block>::Id, <C::Block as Block>::Date>), Error>
    {
        match self.blocks.get(block_hash) {
            None => Err(Error::BlockNotFound),
            Some((block, block_info)) => Ok((C::Block::deserialize(&block[..]).unwrap(), block_info.clone()))
        }
    }

    fn get_block_info(&self, block_hash: &<C::Block as Block>::Id) -> Result<BlockInfo<<C::Block as Block>::Id, <C::Block as Block>::Date>, Error>
    {
        COUNTER.fetch_add(1, Ordering::Relaxed);

        match self.blocks.get(block_hash) {
            None => Err(Error::BlockNotFound),
            Some((_, block_info)) => Ok(block_info.clone())
        }
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &<C::Block as Block>::Id) -> Result<(), Error>
    {
        assert!(self.blocks.get(block_hash).is_some()); // FIXME: return error
        self.tags.insert(tag_name.to_string(), block_hash.clone());
        Ok(())
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<<C::Block as Block>::Id>, Error>
    {
        if let Some(hash) = self.tags.get(tag_name) {
            Ok(Some(hash.clone()))
        } else {
            Ok(None)
        }
    }

    fn get_genesis_hash(&self) -> <C::Block as Block>::Id {
        self.genesis_hash.clone()
    }
}

impl<C> ChainStateStore<C> for MemoryBlockStore<C> where C: ChainState {

    fn get_chain_state_at(&self, block_hash: &<C::Block as Block>::Id) -> Result<C, C::Error> {

        // Iterate backwards to the first block for which we know the chain state.
        let mut blocks_to_apply = vec![];
        let mut cur_hash = block_hash.clone();
        let mut chain_state = loop {
            if cur_hash == self.genesis_hash {
                break self.genesis_chain_state.clone();
            } else {
                if let Some(_) = self.chain_state_deltas.get(&cur_hash) {
                    // Get the chain of deltas.
                    let mut bases = vec![];
                    let mut cur_base = cur_hash.clone();
                    while cur_base != self.genesis_hash {
                        bases.push(cur_base.clone());
                        cur_base = self.chain_state_deltas.get(&cur_base).unwrap().0.clone();
                    };

                    // Apply them to the genesis state.
                    let mut chain_state = self.genesis_chain_state.clone();
                    for base in bases.iter().rev() {
                        chain_state.apply_delta(C::Delta::deserialize(
                            &self.chain_state_deltas.get(base).unwrap().1[..]).unwrap())?;
                    }
                    assert_eq!(chain_state.get_last_block_id(), cur_hash);
                    break chain_state;
                }
                blocks_to_apply.push(cur_hash.clone());
                cur_hash = self.get_block_info(&cur_hash).unwrap().parent_id();
            }
        };

        // Apply the remaining blocks.
        for hash in blocks_to_apply.iter().rev() {
            chain_state.apply_block(&self.get_block(&hash).unwrap().0)?;

            // Store the chain state, if this has not been done
            // previously. FIXME: requires (interior) mutability.
            //self.put_chain_state(chain_state);
        }

        assert_eq!(&chain_state.get_last_block_id(), block_hash);

        Ok(chain_state)
    }

    fn put_chain_state(&mut self, chain_state: &C) -> Result<(), C::Error> {

        let interval = 5000;

        let depth = chain_state.get_chain_length();

        if depth % interval != 0 { return Ok(()); }

        let base_depth = clear_lsb(depth / interval) * interval;

        let base = if base_depth == 0 {
            self.genesis_chain_state.clone()
        } else {
            let base_block = self.get_nth_ancestor(&chain_state.get_last_block_id(), depth - base_depth).unwrap();
            self.get_chain_state_at(&base_block.block_hash)?
        };

        let delta = C::diff(&base, chain_state)?;

        self.chain_state_deltas.insert(
            chain_state.get_last_block_id(),
            (base.get_last_block_id(), delta.serialize_as_vec().unwrap()));

        debug_assert!(chain_state == &self.get_chain_state_at(&chain_state.get_last_block_id()).unwrap());

        Ok(())
    }

}

// Clear the least-significant bit in `x`.
fn clear_lsb(x: u64) -> u64 {
    assert_ne!(x, 0);
    for n in 0..63 {
        if x & (1 << n) != 0 {
            return x & !(1 << n);
        }
    }
    unreachable!();
}
