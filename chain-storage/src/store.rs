use super::error::Error;
use chain_core::property::{Block, BlockId};

#[derive(Clone, Debug)]
pub struct BlockInfo<Id: BlockId> {
    pub block_hash: Id,

    /// Length of the chain. I.e. a block whose parent is the zero
    /// hash has depth 1, its children have depth 2, and so on. Note
    /// that there is no block with depth 0 because there is no block
    /// with the zero hash.
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
        self.get_path_to_nth_ancestor(block_hash, distance, Box::new(|_| {}))
    }

    /// Like get_nth_ancestor(), but calls the closure 'callback' with
    /// each intermediate block encountered while travelling from
    /// 'block_hash' to its n'th ancestor.
    fn get_path_to_nth_ancestor<'a>(
        &'a self,
        block_hash: &<Self::Block as Block>::Id,
        distance: u64,
        mut callback: Box<'a + FnMut(&BlockInfo<<Self::Block as Block>::Id>)>,
    ) -> Result<BlockInfo<<Self::Block as Block>::Id>, Error> {
        let mut cur_block_info = self.get_block_info(block_hash)?;

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
            cur_block_info = self.get_block_info(&best_link.block_hash)?;
        }

        assert_eq!(target, cur_block_info.depth);

        Ok(cur_block_info)
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

    /// Return an iterator that yields block info for the blocks in
    /// the half-open range `(from, to]`. `from` must be an ancestor
    /// of `to` and may be the zero hash.
    fn iterate_range<'store>(
        &'store self,
        from: &<Self::Block as Block>::Id,
        to: &<Self::Block as Block>::Id,
    ) -> Result<BlockIterator<'store, Self::Block>, Error> {
        // FIXME: put blocks loaded by is_ancestor into pending_infos.
        match self.is_ancestor(from, to)? {
            None => Err(Error::CannotIterate),
            Some(distance) => {
                let to_info = self.get_block_info(&to)?;
                Ok(BlockIterator {
                    store: self.as_trait(),
                    to_depth: to_info.depth,
                    cur_depth: to_info.depth - distance,
                    pending_infos: vec![to_info],
                })
            }
        }
    }

    // See https://stackoverflow.com/questions/42121299/provided-method-casting-self-to-trait-object
    fn as_trait(&self) -> &BlockStore<Block = Self::Block>;
}

pub struct BlockIterator<'store, B: Block> {
    store: &'store BlockStore<Block = B>,
    to_depth: u64,
    cur_depth: u64,
    pending_infos: Vec<BlockInfo<B::Id>>,
}

impl<'store, B: Block> Iterator for BlockIterator<'store, B> {
    type Item = Result<BlockInfo<B::Id>, Error>;

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
                Some(self.store.get_path_to_nth_ancestor(
                    &parent,
                    depth - self.cur_depth - 1,
                    Box::new(|new_info| {
                        self.pending_infos.push(new_info.clone());
                    }),
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
