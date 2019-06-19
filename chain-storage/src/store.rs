use super::error::Error;
use chain_core::property::{Block, BlockId};

#[derive(Clone, Debug)]
pub struct BlockInfo<Id: BlockId> {
    pub block_hash: Id,

    /// Length of the chain. I.e. a block whose parent is the zero
    /// hash has depth 1, its children have depth 2, and so on. Note
    /// that there is no block with depth 0 because there is no block
    /// with the zero hash.
    // FIXME: rename to chain_length
    pub depth: u64,

    /// One or more ancestors of this block. Must include at least the
    /// parent, but may include other ancestors to enable efficient
    /// random access in get_nth_ancestor().
    pub back_links: Vec<BackLink<Id>>,
}

impl<Id: BlockId> BlockInfo<Id> {
    pub fn parent_id(&self) -> Id {
        self.back_links
            .iter()
            .find(|x| x.distance == 1)
            .unwrap()
            .block_hash
            .clone()
    }
}

#[derive(Clone, Debug)]
pub struct BackLink<Id: BlockId> {
    /// The distance to this ancestor.
    pub distance: u64,
    /// The hash of the ancestor.
    pub block_hash: Id,
}

pub trait BlockStore {
    type Block: Block;

    /// Write a block to the store. The parent of the block must exist
    /// (unless it's the zero hash).
    ///
    /// The default implementation computes a BlockInfo structure with
    /// back_links set to ensure O(lg n) seek time in
    /// get_nth_ancestor(), and calls put_block_internal() to do the
    /// actual write.
    fn put_block(&mut self, block: &Self::Block) -> Result<(), Error> {
        let block_hash = block.id();

        if self.block_exists(&block_hash)? {
            return Ok(());
        }

        let parent_hash = block.parent_id();

        // Always include a link to the parent.
        let mut back_links = vec![BackLink {
            distance: 1,
            block_hash: parent_hash.clone(),
        }];

        let depth = if parent_hash == <Self::Block as Block>::Id::zero() {
            1
        } else {
            let parent_info = self.get_block_info(&parent_hash)?;
            assert!(parent_info.depth > 0);
            let depth = 1 + parent_info.depth;
            let fast_link = compute_fast_link(depth);
            //println!("from {} -> {}", depth, fast_link);
            let distance = depth - fast_link;
            if distance != 1 && fast_link > 0 {
                let far_block_info = self.get_nth_ancestor(&parent_hash, depth - 1 - fast_link)?;
                back_links.push(BackLink {
                    distance,
                    block_hash: far_block_info.block_hash,
                })
            }

            depth
        };

        self.put_block_internal(
            block,
            BlockInfo {
                block_hash: block_hash.clone(),
                depth,
                back_links,
            },
        )
    }

    /// Write a block and associated info to the store.
    fn put_block_internal(
        &mut self,
        block: &Self::Block,
        block_info: BlockInfo<<Self::Block as Block>::Id>,
    ) -> Result<(), Error>;

    /// Fetch a block.
    fn get_block(
        &self,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<(Self::Block, BlockInfo<<Self::Block as Block>::Id>), Error>;

    /// Fetch a block.
    fn get_block_info(
        &self,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<BlockInfo<<Self::Block as Block>::Id>, Error>;

    /// Check whether a block exists.
    fn block_exists(&self, block_hash: &<Self::Block as Block>::Id) -> Result<bool, Error> {
        match self.get_block_info(block_hash) {
            Ok(_) => Ok(true),
            Err(Error::BlockNotFound) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// Upsert a tag.
    fn put_tag(
        &mut self,
        tag_name: &str,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<(), Error>;

    /// Get a tag, if previously set.
    fn get_tag(&self, tag_name: &str) -> Result<Option<<Self::Block as Block>::Id>, Error>;

    /// Get the n'th ancestor of the specified block.
    fn get_nth_ancestor(
        &self,
        block_hash: &<Self::Block as Block>::Id,
        distance: u64,
    ) -> Result<BlockInfo<<Self::Block as Block>::Id>, Error> {
        for_path_to_nth_ancestor(self, block_hash, distance, |_| {})
    }

    /// Determine whether block 'ancestor' is an ancestor block
    /// 'descendent'. If so, return the chain distance between them.
    fn is_ancestor(
        &self,
        ancestor: &<Self::Block as Block>::Id,
        descendent: &<Self::Block as Block>::Id,
    ) -> Result<Option<u64>, Error> {
        // Optimization.
        if ancestor == descendent {
            return Ok(Some(0));
        }

        let descendent = self.get_block_info(&descendent)?;

        if ancestor == &<Self::Block as Block>::Id::zero() {
            return Ok(Some(descendent.depth));
        }

        let ancestor = self.get_block_info(&ancestor)?;

        // Bail out right away if the "descendent" does not have a
        // higher depth.
        if descendent.depth <= ancestor.depth {
            return Ok(None);
        }

        // Seek back from the descendent to check whether it has the
        // ancestor at the expected place.
        let info =
            self.get_nth_ancestor(&descendent.block_hash, descendent.depth - ancestor.depth)?;

        if info.block_hash == ancestor.block_hash {
            Ok(Some(descendent.depth - ancestor.depth))
        } else {
            Ok(None)
        }
    }
}

impl<T: ?Sized + BlockStore> BlockStore for Box<T> {
    type Block = T::Block;

    fn put_block_internal(
        &mut self,
        block: &Self::Block,
        block_info: BlockInfo<<Self::Block as Block>::Id>,
    ) -> Result<(), Error> {
        (**self).put_block_internal(block, block_info)
    }

    fn get_block(
        &self,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<(Self::Block, BlockInfo<<Self::Block as Block>::Id>), Error> {
        (**self).get_block(block_hash)
    }

    fn get_block_info(
        &self,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<BlockInfo<<Self::Block as Block>::Id>, Error> {
        (**self).get_block_info(block_hash)
    }

    fn block_exists(&self, block_hash: &<Self::Block as Block>::Id) -> Result<bool, Error> {
        (**self).block_exists(block_hash)
    }

    fn put_tag(
        &mut self,
        tag_name: &str,
        block_hash: &<Self::Block as Block>::Id,
    ) -> Result<(), Error> {
        (**self).put_tag(tag_name, block_hash)
    }

    fn get_tag(&self, tag_name: &str) -> Result<Option<<Self::Block as Block>::Id>, Error> {
        (**self).get_tag(tag_name)
    }
}

/// Return an iterator that yields block info for the blocks of `store` in
/// the half-open range `(from, to]`. `from` must be an ancestor
/// of `to` and may be the zero hash.
pub fn iterate_range<'store, S>(
    store: &'store S,
    from: &<S::Block as Block>::Id,
    to: &<S::Block as Block>::Id,
) -> Result<BlockIterator<'store, S>, Error>
where
    S: ?Sized + BlockStore,
{
    // FIXME: put blocks loaded by is_ancestor into pending_infos.
    match store.is_ancestor(from, to)? {
        None => Err(Error::CannotIterate),
        Some(distance) => {
            let to_info = store.get_block_info(&to)?;
            Ok(BlockIterator {
                store: store,
                to_depth: to_info.depth,
                cur_depth: to_info.depth - distance,
                pending_infos: vec![to_info],
            })
        }
    }
}

/// Like `BlockStore::get_nth_ancestor`, but calls the closure 'callback' with
/// each intermediate block encountered while travelling from
/// 'block_hash' to its n'th ancestor.
///
/// The travelling algorithm uses back links to skip over parts of the chain,
/// so the callback will not be invoked for all blocks in the linear sequence.
pub fn for_path_to_nth_ancestor<S, F>(
    store: &S,
    block_hash: &<S::Block as Block>::Id,
    distance: u64,
    mut callback: F,
) -> Result<BlockInfo<<S::Block as Block>::Id>, Error>
where
    S: ?Sized + BlockStore,
    F: FnMut(&BlockInfo<<S::Block as Block>::Id>),
{
    let mut cur_block_info = store.get_block_info(block_hash)?;

    if distance >= cur_block_info.depth {
        // FIXME: return error
        panic!(
            "distance {} > chain length {}",
            distance, cur_block_info.depth
        );
    }

    let target = cur_block_info.depth - distance;

    // Travel back through the chain using the back links until we
    // reach the desired block.
    while target < cur_block_info.depth {
        // We're not there yet. Use the back link that takes us
        // furthest back in the chain, without going beyond the
        // block we're looking for.
        let best_link = cur_block_info
            .back_links
            .iter()
            .filter(|x| cur_block_info.depth - target >= x.distance)
            .max_by_key(|x| x.distance)
            .unwrap()
            .clone();
        callback(&cur_block_info);
        cur_block_info = store.get_block_info(&best_link.block_hash)?;
    }

    assert_eq!(target, cur_block_info.depth);

    Ok(cur_block_info)
}

pub struct BlockIterator<'store, S>
where
    S: ?Sized + BlockStore,
{
    store: &'store S,
    to_depth: u64,
    cur_depth: u64,
    pending_infos: Vec<BlockInfo<<S::Block as Block>::Id>>,
}

impl<'store, S> Iterator for BlockIterator<'store, S>
where
    S: ?Sized + BlockStore,
{
    type Item = Result<BlockInfo<<S::Block as Block>::Id>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cur_depth >= self.to_depth {
            None
        } else {
            self.cur_depth += 1;

            let block_info = self.pending_infos.pop().unwrap();

            if block_info.depth == self.cur_depth {
                // We've seen this block on a previous ancestor traversal.
                Some(Ok(block_info))
            } else {
                // We don't have this block yet, so search back from
                // the furthest block that we do have.
                assert!(self.cur_depth < block_info.depth);
                let depth = block_info.depth;
                let parent = block_info.parent_id();
                self.pending_infos.push(block_info);
                Some(for_path_to_nth_ancestor(
                    self.store,
                    &parent,
                    depth - self.cur_depth - 1,
                    |new_info| {
                        self.pending_infos.push(new_info.clone());
                    },
                ))
            }
        }
    }
}

/// Compute the fast link for a block with a given depth. Successive
/// blocks make a depth jump equal to differents powers of two, minus
/// 1, e.g. 1, 3, 7, 15, 31, ...
fn compute_fast_link(depth: u64) -> u64 {
    let order = depth % 32;
    let distance = if order == 0 { 1 } else { (1 << order) - 1 };
    if distance < depth {
        depth - distance
    } else {
        0
    }
}

#[cfg(any(feature = "test-api", test))]
pub mod testing {
    use super::*;
    use chain_core::packer::*;
    use chain_core::property::{Block as _, BlockDate as _, BlockId as _};
    use rand::Rng;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Copy)]
    pub struct BlockId(pub u64);

    static GLOBAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

    impl BlockId {
        pub fn generate() -> Self {
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
            let mut codec = Codec::new(reader);
            Ok(Self(codec.get_u64()?))
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
        pub fn genesis() -> Self {
            Self {
                id: BlockId::generate(),
                parent: BlockId::zero(),
                date: BlockDate::from_epoch_slot_id(0, 0),
                chain_length: ChainLength(1),
            }
        }

        pub fn make_child(&self) -> Self {
            Self {
                id: BlockId::generate(),
                parent: self.id,
                date: BlockDate::from_epoch_slot_id(self.date.0, self.date.1 + 1),
                chain_length: ChainLength(self.chain_length.0 + 1),
            }
        }
    }

    #[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
    pub struct ChainLength(pub u64);

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

    pub fn generate_chain<Store: BlockStore<Block = Block>>(store: &mut Store) -> Vec<Block> {
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

    pub fn test_put_get<Store: BlockStore<Block = Block>>(store: &mut Store) {
        assert!(store.get_tag("tip").unwrap().is_none());

        match store.put_tag("tip", &BlockId::zero()) {
            Err(Error::BlockNotFound) => {}
            err => panic!(err),
        }

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

    pub fn test_nth_ancestor<Store: BlockStore<Block = Block>>(store: &mut Store) {
        let blocks = generate_chain(store);

        let mut rng = rand::thread_rng();

        let mut blocks_fetched = 0;
        let mut total_distance = 0;
        let nr_tests = 1000;

        for _ in 0..nr_tests {
            let block = &blocks[rng.gen_range(0, blocks.len())];
            assert_eq!(&store.get_block(&block.id()).unwrap().0, block);

            let distance = rng.gen_range(0, block.chain_length().0);
            total_distance += distance;

            let ancestor_info = for_path_to_nth_ancestor(store, &block.id(), distance, |_| {
                blocks_fetched += 1;
            })
            .unwrap();

            assert_eq!(ancestor_info.depth + distance, block.chain_length().0);

            let ancestor = store.get_block(&ancestor_info.block_hash).unwrap().0;

            assert_eq!(ancestor.chain_length().0 + distance, block.chain_length().0);
        }

        let blocks_per_test = blocks_fetched as f64 / nr_tests as f64;

        println!(
            "fetched {} intermediate blocks ({} per test), total distance {}",
            blocks_fetched, blocks_per_test, total_distance
        );

        assert!(blocks_per_test < 35.0);
    }

    pub fn test_iterate_range<Store: BlockStore<Block = Block>>(store: &mut Store) {
        let blocks = generate_chain(store);

        let blocks_by_id: HashMap<BlockId, &Block> = blocks.iter().map(|b| (b.id(), b)).collect();

        let mut rng = rand::thread_rng();

        for _ in 0..1000 {
            let from = &blocks[rng.gen_range(0, blocks.len())];
            let to = &blocks[rng.gen_range(0, blocks.len())];

            match iterate_range(store, &from.id(), &to.id()) {
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
                Err(_) => panic!(),
            }
        }
    }

}
