use super::{Hash, Block, BlockInfo, BackLink, BlockStore, Error};
use std::collections::HashMap;

pub struct MemoryBlockStore<B> where B: Block {
    genesis_hash: Hash,
    // FIXME: store serialized blocks?
    blocks: HashMap<Hash, (Vec<u8>, BlockInfo)>,
    tags: HashMap<String, Hash>,
    dummy: std::marker::PhantomData<B>,
}

impl<B> MemoryBlockStore<B> where B: Block {
    pub fn new(genesis_hash: Hash) -> Self {
        MemoryBlockStore {
            genesis_hash,
            blocks: HashMap::new(),
            tags: HashMap::new(),
            dummy: std::marker::PhantomData,
        }
    }
}

impl<B> BlockStore<B> for MemoryBlockStore<B> where B: Block {

    fn put_block_internal(&mut self, block: B, block_info: BlockInfo) -> Result<(), Error>
    {
        self.blocks.insert(block_info.block_hash.clone(), (block.serialize(), block_info));
        Ok(())
    }

    fn get_block(&self, block_hash: &Hash) -> Result<(B, BlockInfo), Error>
    {
        match self.blocks.get(block_hash) {
            None => Err(cardano_storage::Error::BlockNotFound(block_hash.clone().into())),
            Some((block, block_info)) => Ok((B::deserialize(block), block_info.clone()))
        }
    }

    fn get_block_info(&self, block_hash: &Hash) -> Result<BlockInfo, Error>
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
