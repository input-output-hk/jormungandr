use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chain_core::property::{Block as _, BlockId as _, HasMessages as _};
use chain_impl_mockchain::multiverse;
use chain_storage::{
    error as storage,
    memory::MemoryBlockStore,
    store::{BlockInfo, BlockStore},
};
use chain_storage_sqlite::SQLiteBlockStore;

use crate::blockcfg::{
    genesis_data::GenesisData, Block, HeaderHash, Ledger, LedgerStaticParameters, Multiverse,
};

pub struct Blockchain {
    /// the storage for the overall blockchains (blocks)
    pub storage: Arc<RwLock<Box<BlockStore<Block = Block> + Send + Sync>>>,

    pub multiverse: Multiverse<HeaderHash, Ledger>,

    pub tip: multiverse::GCRoot<HeaderHash>,

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
        let discrimination = genesis_data.address_discrimination.into();

        let mut storage: Box<BlockStore<Block = Block> + Send + Sync>;
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

        let mut multiverse = multiverse::Multiverse::new();

        let tip = if let Some(tip_hash) = storage.get_tag(LOCAL_BLOCKCHAIN_TIP_TAG).unwrap() {
            info!("restoring state at tip {}", tip_hash);

            let mut tip = None;

            let block_0_id = genesis_data.to_block_0().id(); // TODO: get this from the parameter
            let (block_0, _block_0_info) = storage.get_block(&block_0_id).unwrap();
            let parameter_0 = LedgerStaticParameters {
                block0_initial_hash: block_0_id,
                discrimination: discrimination,
            };
            let mut state = Ledger::new(parameter_0, block_0.messages()).unwrap();

            // FIXME: should restore from serialized chain state once we have it.
            for info in storage.iterate_range(&block_0_id, &tip_hash).unwrap() {
                let info = info.unwrap();
                let parameters = state.get_ledger_parameters();
                let block = &storage.get_block(&info.block_hash).unwrap().0;
                state = state.apply_block(&parameters, block.messages()).unwrap();
                tip = Some(multiverse.add(info.block_hash.clone(), state.clone()));
            }

            tip.unwrap()
        } else {
            let block_0 = genesis_data.to_block_0();
            let parameter_0 = LedgerStaticParameters {
                block0_initial_hash: block_0.id(),
                discrimination: discrimination,
            };
            let state = Ledger::new(parameter_0, block_0.messages()).unwrap();
            storage.put_block(&block_0).unwrap();
            multiverse.add(block_0.id(), state)
        };

        multiverse.gc();

        Blockchain {
            storage: Arc::new(RwLock::new(storage)),
            multiverse,
            tip,
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
        if block_hash == *self.tip {
            return;
        }

        let state = self.multiverse.get(&block.parent_id()).unwrap().clone(); // FIXME
        let (block_tip, _block_tip_info) = self
            .storage
            .read()
            .unwrap()
            .get_block(&block.parent_id())
            .unwrap();
        let current_parameters = state.get_ledger_parameters();

        let tip_chain_length = block_tip.chain_length();

        match state.apply_block(&current_parameters, block.messages()) {
            Ok(state) => {
                // FIXME: currently we store all incoming blocks and
                // corresponding states, but to prevent a DoS, we may
                // want to store only sufficiently long chains.

                let mut storage = self.storage.write().unwrap();
                storage.put_block(&block).unwrap();
                storage
                    .put_tag(LOCAL_BLOCKCHAIN_TIP_TAG, &block_hash)
                    .unwrap();

                let new_chain_length = block.chain_length();

                let tip = self.multiverse.add(block_hash, state);

                if new_chain_length > tip_chain_length {
                    self.tip = tip;
                }
            }
            Err(error) => error!("Error with incoming block: {}", error),
        }
    }

    /// return the current tip hash and date
    pub fn get_tip(&self) -> HeaderHash {
        self.tip.clone()
    }

    pub fn get_block_tip(
        &self,
    ) -> Result<(Block, BlockInfo<HeaderHash>), chain_storage::error::Error> {
        self.storage.read().unwrap().get_block(&self.tip)
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
    fn sollicit_block(&mut self, block_hash: &HeaderHash) {
        info!("solliciting block {} from the network", block_hash);
        //unimplemented!();
    }

    pub fn handle_block_announcement(&mut self, _header: HeaderHash) -> Result<(), storage::Error> {
        info!("received block announcement, handling not implemented yet");
        Ok(())
    }
}
