use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chain_core::property::{
    Block as _, BlockId as _, HasHeader as _, HasMessages as _, Settings as _, State as _,
};
use chain_impl_mockchain::state::State;
use chain_storage::{error as storage, memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;

use crate::blockcfg::{genesis_data::GenesisData, Block, HeaderHash, Ledger, MultiVerse};

pub struct Blockchain {
    /// the storage for the overall blockchains (blocks)
    pub storage: Arc<RwLock<Box<BlockStore<Block = Block> + Send + Sync>>>,

    // TODO use multiverse here
    pub multiverse: MultiVerse<Ledger>,

    /// Incoming blocks whose parent does not exist yet. Sorted by
    /// parent hash to allow quick look up of the children of a
    /// parent.
    ///
    /// FIXME: need some way to GC unconnected blocks after a while.
    pub unconnected_blocks: BTreeMap<HeaderHash, BTreeMap<HeaderHash, Block>>,
}

pub type BlockchainR = Arc<RwLock<Blockchain>>;

// FIXME: copied from cardano-cli
pub const LOCAL_BLOCKCHAIN_TIP_TAG: &'static str = "tip";

impl Blockchain {
    pub fn new(genesis_data: GenesisData, storage_dir: &Option<std::path::PathBuf>) -> Self {
        let mut state = State::new(genesis_data.address_discrimination.into());

        let mut storage: Box<BlockStore<Block> + Send + Sync>;
        match storage_dir {
            None => {
                info!("storing blockchain in memory");
                storage = Box::new(MemoryBlockStore::new());
            }
            Some(dir) => {
                std::fs::create_dir_all(dir).unwrap();
                let mut sqlite = dir.clone();
                sqlite.push("blocks.sqlite");
                let path = sqlite.to_str().unwrap();
                info!("storing blockchain in '{}'", path);
                storage = Box::new(SQLiteBlockStore::new(path));
            }
        };

        if let Some(tip_hash) = storage.get_tag(LOCAL_BLOCKCHAIN_TIP_TAG).unwrap() {
            info!("restoring state at tip {}", tip_hash);

            // FIXME: should restore from serialized chain state once we have it.
            for info in storage
                .iterate_range(&HeaderHash::zero(), &tip_hash)
                .unwrap()
            {
                let info = info.unwrap();
                let block = &storage.get_block(&info.block_hash).unwrap().0;
                state = state.apply_block(block.messages()).unwrap();
            }
        } else {
            let block_0 = genesis_data.to_block_0();
            state = state.apply_block(block_0.messages()).unwrap();
            storage.put_block(&block_0).unwrap();
        }

        Blockchain {
            storage: Arc::new(RwLock::new(storage)),
            state: state,
            unconnected_blocks: BTreeMap::default(),
        }
    }

    pub fn handle_incoming_block(&mut self, block: Block) -> Result<(), storage::Error> {
        let block_hash = block.id();
        let parent_hash = block.parent_id();

        if parent_hash == HeaderHash::zero() || self.block_exists(&parent_hash)? {
            self.handle_connected_block(block_hash, block);
        } else {
            self.sollicit_block(&parent_hash);
            self.unconnected_blocks
                .entry(parent_hash)
                .or_insert(BTreeMap::new())
                .insert(block_hash, block);
        }
        Ok(())
    }

    /// Handle a block whose ancestors are on disk.
    fn handle_connected_block(&mut self, block_hash: HeaderHash, block: Block) {
        let current_tip = self
            .storage
            .get_tag(LOCAL_BLOCKCHAIN_TIP_TAG)
            .unwrap()
            .unwrap();

        let current_ledger = self.multiverse.get(current_tip);

        match current_ledger.apply_block(&block.header(), block.messages()) {
            Ok(state) => {
                if block_hash != current_tip {
                    let mut storage = self.storage.write().unwrap();
                    storage.put_block(&block).unwrap();
                    storage
                        .put_tag(LOCAL_BLOCKCHAIN_TIP_TAG, &block_hash)
                        .unwrap();
                }

                // TODO: update with the multiverse here
                self.state = state;
            }
            Err(error) => error!("Error with incoming block: {}", error),
        }
    }

    fn block_exists(&self, block_hash: &HeaderHash) -> Result<bool, storage::Error> {
        // TODO: we assume as an invariant that if a block exists on
        // disk, its ancestors exist on disk as well. Need to make
        // sure that this invariant is preserved everywhere
        // (e.g. loose block GC should delete blocks in reverse
        // order).
        self.storage.read().unwrap().block_exists(block_hash)
    }

    /// Request a missing block from the network.
    fn sollicit_block(&mut self, _block_hash: &HeaderHash) {
        //unimplemented!();
    }

    pub fn handle_block_announcement(&mut self, _header: HeaderHash) -> Result<(), storage::Error> {
        info!("received block announcement, handling not implemented yet");
        Ok(())
    }
}
