use super::error::Error;
use super::store::{BlockInfo, BlockStore};
use chain_core::property::Block;
use std::collections::HashMap;

pub struct MemoryBlockStore<B>
where
    B: Block,
{
    blocks: HashMap<B::Id, (Vec<u8>, BlockInfo<B::Id>)>,
    tags: HashMap<String, B::Id>,
}

impl<B> MemoryBlockStore<B>
where
    B: Block,
{
    pub fn new() -> Self {
        MemoryBlockStore {
            blocks: HashMap::new(),
            tags: HashMap::new(),
        }
    }
}

impl<B> BlockStore for MemoryBlockStore<B>
where
    B: Block,
{
    type Block = B;

    fn put_block_internal(&mut self, block: &B, block_info: BlockInfo<B::Id>) -> Result<(), Error> {
        self.blocks.insert(
            block_info.block_hash.clone(),
            (block.serialize_as_vec().unwrap(), block_info),
        );
        Ok(())
    }

    fn get_block(&self, block_hash: &B::Id) -> Result<(B, BlockInfo<B::Id>), Error> {
        match self.blocks.get(block_hash) {
            None => Err(Error::BlockNotFound),
            Some((block, block_info)) => {
                Ok((B::deserialize(&block[..]).unwrap(), block_info.clone()))
            }
        }
    }

    fn get_block_info(&self, block_hash: &B::Id) -> Result<BlockInfo<B::Id>, Error> {
        match self.blocks.get(block_hash) {
            None => Err(Error::BlockNotFound),
            Some((_, block_info)) => Ok(block_info.clone()),
        }
    }

    fn put_tag(&mut self, tag_name: &str, block_hash: &B::Id) -> Result<(), Error> {
        match self.blocks.get(block_hash) {
            None => Err(Error::BlockNotFound),
            Some(_) => {
                self.tags.insert(tag_name.to_string(), block_hash.clone());
                Ok(())
            }
        }
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<B::Id>, Error> {
        if let Some(hash) = self.tags.get(tag_name) {
            Ok(Some(hash.clone()))
        } else {
            Ok(None)
        }
    }

    fn as_trait(&self) -> &BlockStore<Block = Self::Block> {
        self as &BlockStore<Block = Self::Block>
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::store::test::Block;

    #[test]
    pub fn put_get() {
        let mut store = MemoryBlockStore::<Block>::new();
        crate::store::test::test_put_get(&mut store);
    }

    #[test]
    pub fn nth_ancestor() {
        let mut store = MemoryBlockStore::<Block>::new();
        crate::store::test::test_nth_ancestor(&mut store);
    }

    #[test]
    pub fn iterate_range() {
        let mut store = MemoryBlockStore::<Block>::new();
        crate::store::test::test_iterate_range(&mut store);
    }
}
