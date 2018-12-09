pub mod memory;
pub mod sqlite;

use cardano;
use cardano_storage;
use super::blockchain::{Block, Hash};

type Error = cardano_storage::Error; // FIXME

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub block_hash: Hash,

    /// Distance to the genesis hash (a.k.a chain length). I.e. a
    /// block whose parent is the genesis hash has depth 1, its
    /// children have depth 2, and so on. Note that there is no block
    /// with depth 0 because there is no block with the genesis hash.
    pub depth: u64,

    /// One or more ancestors of this block. Must include at least the
    /// parent, but may include other ancestors to enable efficient
    /// random access in get_nth_ancestor().
    pub back_links: Vec<BackLink>,
}

#[derive(Clone, Debug)]
pub struct BackLink {
    /// The depth delta to this ancestor.
    pub delta: u64,
    /// The hash of the ancestor.
    pub block_hash: Hash,
}

pub trait BlockStore<B> where B: Block {

    /// Write a block to the store. The parent of the block must exist
    /// (unless it's the genesis hash).
    ///
    /// The default implementation computes a BlockInfo structure with
    /// back_links set to ensure O(lg n) seek time in
    /// get_nth_ancestor(), and calls put_block_internal() to do the
    /// actual write.
    fn put_block(&mut self, block: B) -> Result<(), Error> {
        let block_hash = block.get_hash();

        if self.block_exists(&block_hash)? { return Ok(()); }

        let parent_hash = block.get_parent();

        // Always include a link to the parent.
        let mut back_links = vec![BackLink {
            delta: 1,
            block_hash: parent_hash,
        }];

        let depth =
            if parent_hash == self.get_genesis_hash() {
                1
            } else {
                let parent_info = self.get_block_info(&parent_hash)?;
                assert!(parent_info.depth > 0);
                let depth = 1 + parent_info.depth;
                let fast_link = compute_fast_link(depth);
                //println!("from {} -> {}", depth, fast_link);
                let delta = depth - fast_link;
                if delta != 1 && fast_link > 0 {
                    let far_block_info = self.get_nth_ancestor(&parent_hash, depth - 1 - fast_link)?;
                    back_links.push(BackLink {
                        delta,
                        block_hash: far_block_info.block_hash
                    })
                }

                depth
            };

        self.put_block_internal(block, BlockInfo {
            block_hash: block_hash.clone(),
            depth,
            back_links
        })
    }

    /// Write a block and associated info to the store.
    fn put_block_internal(&mut self, block: B, block_info: BlockInfo) -> Result<(), Error>;

    /// Fetch a block.
    fn get_block(&self, block_hash: &Hash) -> Result<(B, BlockInfo), Error>;

    /// Fetch a block.
    fn get_block_info(&self, block_hash: &Hash) -> Result<BlockInfo, Error>;

    /// Check whether a block exists.
    fn block_exists(&self, block_hash: &Hash) -> Result<bool, Error> {
        match self.get_block_info(block_hash) {
            Ok(_) => Ok(true),
            Err(cardano_storage::Error::BlockNotFound(_)) => Ok(false),
            Err(err) => Err(err)
        }
    }

    /// Upsert a tag.
    fn put_tag(&mut self, tag_name: &str, block_hash: &Hash) -> Result<(), Error>;

    /// Get a tag, if previously set.
    fn get_tag(&self, tag_name: &str) -> Result<Option<Hash>, Error>;

    /// Get the n'th ancestor of the specified block.
    fn get_nth_ancestor(&self, block_hash: &Hash, delta: u64) -> Result<BlockInfo, Error>
    {
        let mut cur_block_info = self.get_block_info(block_hash)?;

        if delta >= cur_block_info.depth {
            // FIXME: return error
            panic!("delta {} > chain length {}", delta, cur_block_info.depth);
        }

        let target = cur_block_info.depth - delta;
        let mut nr_steps = 1;

        // Travel back through the chain using the back links until we
        // reach the desired block.
        while target < cur_block_info.depth {
            // We're not there yet. Use the back link that takes us
            // furthest back in the chain, without going beyond the
            // block we're looking for.
            let best_link =
                cur_block_info.back_links.iter()
                .filter(|x| cur_block_info.depth - target >= x.delta)
                .max_by_key(|x| x.delta)
                .unwrap()
                .clone();
            cur_block_info = self.get_block_info(&best_link.block_hash)?;
            nr_steps += 1;
        }

        assert_eq!(target, cur_block_info.depth);

        Ok(cur_block_info)
    }

    fn get_genesis_hash(&self) -> Hash;
}

/// Compute the fast link for a block with a given depth. Successive
/// blocks make a depth jump equal to differents powers of two, minus
/// 1, e.g. 1, 3, 7, 15, 31, ...
fn compute_fast_link(depth: u64) -> u64 {
    let order = depth % 32;
    let delta = if order == 0 { 1 } else { (1 << order) - 1 };
    if delta < depth { depth - delta } else { 0 }
}
