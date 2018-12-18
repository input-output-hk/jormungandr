use super::{Hash, Block, BlockInfo, BlockStore, Error, ChainState, ChainStateStore};
use std::collections::HashMap;

pub struct MemoryBlockStore<B, C> where B: Block, C: ChainState<Block = B> {
    genesis_hash: Hash,
    genesis_chain_state: C,
    // FIXME: store serialized blocks?
    blocks: HashMap<Hash, (Vec<u8>, BlockInfo<B>)>,
    tags: HashMap<String, Hash>,
    phantom1: std::marker::PhantomData<B>,
    phantom2: std::marker::PhantomData<C>,
}

impl<B, C> MemoryBlockStore<B, C> where B: Block, C: ChainState<Block = B> {
    pub fn new(genesis_data: &C::GenesisData) -> Self {
        let genesis_chain_state = C::new(genesis_data).unwrap();
        MemoryBlockStore {
            genesis_hash: genesis_chain_state.get_last_block(),
            genesis_chain_state,
            blocks: HashMap::new(),
            tags: HashMap::new(),
            phantom1: std::marker::PhantomData,
            phantom2: std::marker::PhantomData,
        }
    }
}

impl<B, C> BlockStore<B> for MemoryBlockStore<B, C> where B: Block, C: ChainState<Block = B> {

    fn put_block_internal(&mut self, block: B, block_info: BlockInfo<B>) -> Result<(), Error>
    {
        self.blocks.insert(block_info.block_hash.clone(), (block.serialize(), block_info));
        Ok(())
    }

    fn get_block(&self, block_hash: &Hash) -> Result<(B, BlockInfo<B>), Error>
    {
        match self.blocks.get(block_hash) {
            None => Err(cardano_storage::Error::BlockNotFound(block_hash.clone().into())),
            Some((block, block_info)) => Ok((B::deserialize(block), block_info.clone()))
        }
    }

    fn get_block_info(&self, block_hash: &Hash) -> Result<BlockInfo<B>, Error>
    {
        match self.blocks.get(block_hash) {
            None => Err(cardano_storage::Error::BlockNotFound(block_hash.clone().into())),
            Some((_, block_info)) => Ok(block_info.clone())
        }
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &Hash) -> Result<(), Error>
    {
        assert!(self.blocks.get(block_hash).is_some()); // FIXME: return error
        self.tags.insert(tag_name.to_string(), block_hash.clone());
        Ok(())
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<Hash>, Error>
    {
        if let Some(hash) = self.tags.get(tag_name) {
            Ok(Some(hash.clone()))
        } else {
            Ok(None)
        }
    }

    fn get_genesis_hash(&self) -> Hash {
        self.genesis_hash.clone()
    }
}

impl<B, C> ChainStateStore<C> for MemoryBlockStore<B, C> where B: Block, C: ChainState<Block = B> {

    fn get_chain_state_at(&self, block_hash: &Hash) -> Result<C, C::Error> {

        let mut chain_state = self.genesis_chain_state.clone();

        for block_info in self.iterate_range(&self.genesis_hash, block_hash).unwrap() {
            let block_info = block_info.unwrap();
            chain_state.apply_block(&self.get_block(&block_info.block_hash).unwrap().0)?;
        }

        Ok(chain_state)
    }

}
