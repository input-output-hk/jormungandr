use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;
use std::ops::Bound::Included;
use std::str::FromStr;

use cardano_storage::StorageConfig;
use cardano_storage::{tag, Storage, blob, block_read};
use cardano_storage::chain_state::restore_chain_state;
use cardano::block::{ChainState, Block, BlockDate};

use super::super::blockcfg::{GenesisData, BlockHash};

#[allow(dead_code)]
pub struct Blockchain {
    genesis_data: GenesisData,

    /// the storage for the overall blockchains (blocks)
    storage: Storage,

    /// The current chain state corresponding to our tip.
    chain_state: ChainState,

    /// Incoming blocks whose parent does not exist yet. Sorted by
    /// parent hash to allow quick look up of the children of a
    /// parent.
    ///
    /// FIXME: need some way to GC unconnected blocks after a while.
    unconnected_blocks: BTreeMap<UnconnectedBlockKey, Block>,
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct UnconnectedBlockKey {
    parent_hash: BlockHash,
    block_hash: BlockHash,
}

pub type BlockchainR = Arc<RwLock<Blockchain>>;

// FIXME: copied from cardano-cli
pub const LOCAL_BLOCKCHAIN_TIP_TAG : &'static str = "tip";

impl Blockchain {
    pub fn from_storage(genesis_data: GenesisData, storage_config: &StorageConfig) -> Self {
        let storage = Storage::init(storage_config).unwrap();
        let tip = tag::read_hash(&storage, &LOCAL_BLOCKCHAIN_TIP_TAG).unwrap_or(genesis_data.genesis_prev.clone());
        let chain_state = restore_chain_state(&storage, &genesis_data, &tip)
            .expect("restoring chain state");
        Blockchain {
            genesis_data,
            storage,
            chain_state,
            unconnected_blocks: BTreeMap::new(),
        }
    }

    /// return the current tip hash and date
    pub fn get_tip(&self) -> (BlockHash, BlockDate) {
        (self.chain_state.last_block.clone(), self.chain_state.last_date.unwrap())
    }

    pub fn get_storage(&self) -> &Storage {
        &self.storage
    }

    pub fn get_genesis_hash(&self) -> &BlockHash {
        &self.genesis_data.genesis_prev
    }

    /// Handle an incoming block (either from the network or from our
    /// own leadership task). If the block is not connected, then
    /// sollicit its parent. If it is connected and is a longer valid
    /// chain than the current tip, then switch the tip. If it is
    /// connected but is not a longer valid chain, then discard it.
    pub fn handle_incoming_block(&mut self, block: Block) {

        let block_hash = block.get_header().compute_hash();
        let parent_hash = block.get_header().get_previous_header();

        if self.block_exists(&parent_hash) {
            self.handle_connected_block(block_hash, parent_hash, block);
        } else {
            self.sollicit_block(&parent_hash);
            self.unconnected_blocks.insert(
                UnconnectedBlockKey {
                    parent_hash,
                    block_hash,
                }, block);
        }
    }

    /// Handle a block whose ancestors are on disk.
    fn handle_connected_block(&mut self, block_hash: BlockHash, parent_hash: BlockHash, block: Block) {
        blob::write(&self.storage, &block_hash, cbor!(block).unwrap().as_ref())
            .expect("unable to write block to disk");

        // FIXME: optimize for the case where new_tip is a child of
        // the current tip. In that case we can clone chain_state and
        // apply the new blocks.
        match restore_chain_state(&self.storage, &self.genesis_data, &block_hash) {
            Ok(new_chain_state) => {
                assert_eq!(new_chain_state.last_block, block_hash);
                if new_chain_state.chain_length > self.chain_state.chain_length {
                    info!("switching to new tip {} ({:?}), previous length {}, new length {}",
                          block_hash, new_chain_state.last_date,
                          self.chain_state.chain_length, new_chain_state.chain_length);
                    self.chain_state = new_chain_state;
                } else {
                    info!("discarding shorter incoming fork {} ({:?}, length {}), tip length {}",
                          block_hash, new_chain_state.last_date,
                          new_chain_state.chain_length, self.chain_state.chain_length);
                }
            }
            Err(err) => error!("cannot compute chain state for incoming fork {}: {}", block_hash, err)
        }

        // Process previously received children of this block.
        let from = UnconnectedBlockKey {
            parent_hash: parent_hash.clone(),
            block_hash: BlockHash::from_str(&"0000000000000000000000000000000000000000000000000000000000000000").unwrap()
        };

        let to = UnconnectedBlockKey {
            parent_hash: parent_hash.clone(),
            block_hash: BlockHash::from_str(&"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").unwrap()
        };

        let mut children = vec![];

        for (k, v) in self.unconnected_blocks.range((Included(from), Included(to))) {
            assert_eq!(k.parent_hash, block_hash);
            info!("triggering child block {}", k.block_hash);
            children.push(k.clone())
        }

        for k in children {
            if let Some(v) = self.unconnected_blocks.remove(&k) {
                self.handle_connected_block(k.block_hash.clone(), k.parent_hash.clone(), block.clone());
            }
        }
    }

    fn block_exists(&self, block_hash: &BlockHash) -> bool {
        // TODO: we assume as an invariant that if a block exists on
        // disk, its ancestors exist on disk as well. Need to make
        // sure that this invariant is preserved everywhere
        // (e.g. loose block GC should delete blocks in reverse
        // order).
        block_read(&self.storage, block_hash).is_some()
    }

    /// Request a missing block from the network.
    fn sollicit_block(&mut self, block_hash: &BlockHash) {
        unimplemented!();
    }
}
