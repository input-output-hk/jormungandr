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
mod test {
    use super::*;
    use chain_core::packer::*;
    use chain_core::property::{Block as _, BlockDate as _, BlockId as _};
    use std::sync::atomic::{AtomicU64, Ordering};
    use rand::Rng;

    #[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Copy)]
    pub struct BlockId(pub u64);

    static GLOBAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

    impl BlockId {
        fn generate() -> Self {
            Self(GLOBAL_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
        }
    }

    impl chain_core::property::BlockId for BlockId {
        fn zero() -> Self {
            Self(0)
        }
    }

    impl chain_core::property::Serialize for BlockId {
        type Error = std::io::Error;

        fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
            let mut codec = Codec::new(writer);
            codec.put_u64(self.0)
        }
    }

    impl chain_core::property::Deserialize for BlockId {
        type Error = std::io::Error;

        fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
            panic!()
        }
    }

    #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
    pub struct BlockDate(u32, u32);

    impl chain_core::property::BlockDate for BlockDate {
        fn from_epoch_slot_id(epoch: u32, slot_id: u32) -> Self {
            Self(epoch, slot_id)
        }
    }

    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct Block {
        id: BlockId,
        parent: BlockId,
        date: BlockDate,
        chain_length: ChainLength,
    }

    impl Block {
        fn genesis() -> Self {
            Self {
                id: BlockId::generate(),
                parent: BlockId::zero(),
                date: BlockDate::from_epoch_slot_id(0, 0),
                chain_length: ChainLength(1),
            }
        }

        fn make_child(&self) -> Self {
            Self {
                id: BlockId::generate(),
                parent: self.id,
                date: BlockDate::from_epoch_slot_id(self.date.0, self.date.1 + 1),
                chain_length: ChainLength(self.chain_length.0 + 1),
            }
        }
    }

    #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
    pub struct ChainLength(u64);

    impl chain_core::property::ChainLength for ChainLength {
        fn next(&self) -> Self {
            Self(self.0 + 1)
        }
    }

    impl chain_core::property::Block for Block {
        type Id = BlockId;
        type Date = BlockDate;
        type ChainLength = ChainLength;
        type Version = u8;

        fn id(&self) -> Self::Id {
            self.id
        }

        fn parent_id(&self) -> Self::Id {
            self.parent
        }

        fn date(&self) -> Self::Date {
            self.date
        }

        fn version(&self) -> Self::Version {
            0
        }

        fn chain_length(&self) -> Self::ChainLength {
            self.chain_length
        }
    }

    impl chain_core::property::Serialize for Block {
        type Error = std::io::Error;

        fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
            let mut codec = Codec::new(writer);
            codec.put_u64(self.id.0)?;
            codec.put_u64(self.parent.0)?;
            codec.put_u32(self.date.0)?;
            codec.put_u32(self.date.1)?;
            codec.put_u64(self.chain_length.0)?;
            Ok(())
        }
    }

    impl chain_core::property::Deserialize for Block {
        type Error = std::io::Error;

        fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
            let mut codec = Codec::new(reader);
            Ok(Self {
                id: BlockId(codec.get_u64()?),
                parent: BlockId(codec.get_u64()?),
                date: BlockDate(codec.get_u32()?, codec.get_u32()?),
                chain_length: ChainLength(codec.get_u64()?),
            })
        }
    }

    #[test]
    pub fn put_get() {
        let mut store = MemoryBlockStore::<Block>::new();

        assert!(store.get_tag("tip").unwrap().is_none());

        assert_eq!(
            store.put_tag("tip", &BlockId::zero()),
            Err(Error::BlockNotFound)
        );

        let genesis_block = Block::genesis();
        store.put_block(&genesis_block).unwrap();
        let (genesis_block_restored, block_info) = store.get_block(&genesis_block.id()).unwrap();
        assert_eq!(genesis_block, genesis_block_restored);
        assert_eq!(block_info.block_hash, genesis_block.id());
        assert_eq!(block_info.depth, genesis_block.chain_length().0);
        assert_eq!(block_info.back_links.len(), 1);
        assert_eq!(block_info.parent_id(), BlockId::zero());

        store.put_tag("tip", &genesis_block.id()).unwrap();
        assert_eq!(store.get_tag("tip").unwrap().unwrap(), genesis_block.id());
    }

    pub fn generate_chain(store: &mut MemoryBlockStore<Block>) -> Vec<Block> {
        let mut blocks = vec![];

        let genesis_block = Block::genesis();
        store.put_block(&genesis_block).unwrap();
        blocks.push(genesis_block);

        let mut rng = rand::thread_rng();

        for _ in 0..10 {
            let mut parent_block = blocks[rng.gen_range(0, blocks.len())].clone();
            for _ in 0..rng.gen_range(1, 10000) {
                let block = parent_block.make_child();
                store.put_block(&block).unwrap();
                parent_block = block.clone();
                blocks.push(block);
            }
        }

        blocks
    }

    #[test]
    pub fn nth_ancestor() {
        let mut store = MemoryBlockStore::<Block>::new();

        let blocks = generate_chain(&mut store);

        let blocks_by_id: HashMap<BlockId, &Block> = blocks.iter().map(|b| (b.id(), b)).collect();

        let mut rng = rand::thread_rng();

        let mut blocks_fetched = 0;
        let mut total_distance = 0;
        let nr_tests = 1000;

        for _ in 0..nr_tests {
            let block = &blocks[rng.gen_range(0, blocks.len())];
            assert_eq!(&store.get_block(&block.id()).unwrap().0, block);

            let distance = rng.gen_range(0, block.chain_length().0);
            total_distance += distance;

            let ancestor_info = store.get_path_to_nth_ancestor(&block.id(), distance,
                                                               Box::new(|_| { blocks_fetched += 1; })).unwrap();

            assert_eq!(ancestor_info.depth + distance, block.chain_length().0);

            let ancestor = store.get_block(&ancestor_info.block_hash).unwrap().0;

            assert_eq!(ancestor.chain_length().0 + distance, block.chain_length().0);
        }

        println!("fetched {} intermediate blocks ({} per test), total distance {}", blocks_fetched, blocks_fetched as f64 / nr_tests as f64, total_distance);
    }

    #[test]
    pub fn iterate_range() {
        let mut store = MemoryBlockStore::<Block>::new();

        let blocks = generate_chain(&mut store);

        let blocks_by_id: HashMap<BlockId, &Block> = blocks.iter().map(|b| (b.id(), b)).collect();

        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let from = &blocks[rng.gen_range(0, blocks.len())];
            let to = &blocks[rng.gen_range(0, blocks.len())];

            match store.iterate_range(&from.id(), &to.id()) {
                Ok(iter) => {
                    let mut prev = from.id();
                    for block_info in iter {
                        let block_info = block_info.unwrap();
                        assert_eq!(block_info.parent_id(), prev);
                        prev = block_info.block_hash;
                    }
                    assert_eq!(prev, to.id());
                }
                Err(Error::CannotIterate) => {
                    // Check that 'from' really isn't an ancestor of 'to'.
                    let mut cur = to.id();
                    while cur != BlockId::zero() {
                        assert_ne!(cur, from.id());
                        cur = blocks_by_id[&cur].parent_id();
                    }
                }
                Err(_) => panic!()
            }
        }
    }
}
