use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chain_core::property::{
    Block as _, BlockId as _, HasHeader as _, HasMessages as _, Settings as _, State as _,
};
use chain_impl_mockchain::state::State;
use chain_storage::{error as storage, memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;

use crate::blockcfg::{genesis_data::GenesisData, mock::Mockchain, BlockConfig};

pub struct Blockchain<B: BlockConfig> {
    /// the storage for the overall blockchains (blocks)
    pub storage: Arc<RwLock<Box<BlockStore<Block = B::Block> + Send + Sync>>>,

    // TODO use multiverse here
    pub state: B::State,

    /// Incoming blocks whose parent does not exist yet. Sorted by
    /// parent hash to allow quick look up of the children of a
    /// parent.
    ///
    /// FIXME: need some way to GC unconnected blocks after a while.
    pub unconnected_blocks: BTreeMap<B::BlockHash, BTreeMap<B::BlockHash, B::Block>>,
}

pub type BlockchainR<B> = Arc<RwLock<Blockchain<B>>>;

// FIXME: copied from cardano-cli
pub const LOCAL_BLOCKCHAIN_TIP_TAG: &'static str = "tip";

/*
impl State<Mockchain> {
    pub fn new(genesis: &GenesisData) -> Self {
        let ledger = Arc::new(RwLock::new(ledger::Ledger::new(genesis.initial_utxos())));

        let settings = Arc::new(RwLock::new(setting::Settings::new()));

        let leaders = leadership::genesis::GenesisLeaderSelection::new(
            genesis.leaders().cloned().collect(),
            ledger.clone(),
            settings.clone(),
            HashSet::new(), // initial_stake_pools
            HashMap::new(), // initial_stake_keys
        )
        .unwrap();

        State {
            ledger,
            settings,
            leaders: leaders,
        }
    }
}*/

impl Blockchain<Mockchain> {
    pub fn new(genesis_data: GenesisData, storage_dir: &Option<std::path::PathBuf>) -> Self {
        let mut state = State::new();

        let mut storage: Box<BlockStore<Block = <Mockchain as BlockConfig>::Block> + Send + Sync>;
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
            debug!("restoring state at tip {}", tip_hash);

            // FIXME: should restore from serialized chain state once we have it.
            for info in storage
                .iterate_range(&<Mockchain as BlockConfig>::BlockHash::zero(), &tip_hash)
                .unwrap()
            {
                let info = info.unwrap();
                let block = &storage.get_block(&info.block_hash).unwrap().0;
                state = state.apply_block(&block.header, block.messages()).unwrap();
            }
        } else {
            let block_0 = genesis_data.to_block_0();
            state = state
                .apply_block(&block_0.header, block_0.messages())
                .unwrap();
            storage.put_block(&block_0).unwrap();
        }

        Blockchain {
            storage: Arc::new(RwLock::new(storage)),
            state: state,
            unconnected_blocks: BTreeMap::default(),
        }
    }
}

impl<B: BlockConfig> Blockchain<B> {
    pub fn handle_incoming_block(&mut self, block: B::Block) -> Result<(), storage::Error> {
        let block_hash = block.id();
        let parent_hash = block.parent_id();

        if parent_hash == B::BlockHash::zero() || self.block_exists(&parent_hash)? {
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
    fn handle_connected_block(&mut self, block_hash: B::BlockHash, block: B::Block) {
        let current_tip = self.state.tip();

        match self.state.apply_block(&block.header(), block.messages()) {
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

    /// return the current tip hash and date
    pub fn get_tip(&self) -> B::BlockHash {
        self.state.tip()
    }
}
impl<B: BlockConfig> Blockchain<B> {
    fn block_exists(&self, block_hash: &B::BlockHash) -> Result<bool, storage::Error> {
        // TODO: we assume as an invariant that if a block exists on
        // disk, its ancestors exist on disk as well. Need to make
        // sure that this invariant is preserved everywhere
        // (e.g. loose block GC should delete blocks in reverse
        // order).
        self.storage.read().unwrap().block_exists(block_hash)
    }

    /// Request a missing block from the network.
    fn sollicit_block(&mut self, _block_hash: &B::BlockHash) {
        //unimplemented!();
    }

    pub fn handle_block_announcement(
        &mut self,
        _header: B::BlockHeader,
    ) -> Result<(), storage::Error> {
        info!("received block announcement, handling not implemented yet");
        Ok(())
    }
}

/*
FIXME: we need to restore this when possible

impl Blockchain<Cardano> {
    pub fn from_storage(genesis_data: GenesisData, storage_config: &StorageConfig) -> Self {
        let storage = Storage::init(storage_config).unwrap();
        let tip = tag::read_hash(&storage, &LOCAL_BLOCKCHAIN_TIP_TAG)
            .unwrap_or(genesis_data.genesis_prev.clone());
        let chain_state =
            restore_chain_state(&storage, &genesis_data, &tip).expect("restoring chain state");
        Blockchain {
            genesis_data,
            storage,
            chain_state,
            unconnected_blocks: BTreeMap::new(),
        }
    }

    /// return the current tip hash and date
    pub fn get_tip(&self) -> BlockHash {
        self.chain_state.last_block.clone()
    }

    pub fn get_storage(&self) -> &Storage {
        &self.storage
    }

    pub fn get_genesis_hash(&self) -> &BlockHash {
        &self.genesis_data.genesis_prev
    }

    /// Handle a block whose ancestors are on disk.
    fn handle_connected_block(&mut self, block_hash: BlockHash, block: Block) {
        // Quick optimization: don't do anything if the incoming block
        // is already the tip. Ideally we would bail out if the
        // incoming block is on the tip chain, but there is no quick
        // way to check that.
        if block_hash != self.chain_state.last_block {
            blob::write(
                &self.storage,
                block_hash.as_hash_bytes(),
                cbor!(block).unwrap().as_ref(),
            )
            .expect("unable to write block to disk");

            // Compute the new chain state. In the common case, the
            // incoming block is a direct successor of the tip, so we
            // just apply the block to our current chain
            // state. Otherwise we use restore_chain_state() to
            // compute the chain state from the last state snapshot on
            // disk.
            let new_chain_state =
                if block.get_header().get_previous_header() == self.chain_state.last_block {
                    let mut new_chain_state = self.chain_state.clone();
                    match new_chain_state.verify_block(&block_hash, &block) {
                        Ok(()) => Ok(new_chain_state),
                        Err(err) => Err(err.into()),
                    }
                } else {
                    restore_chain_state(&self.storage, &self.genesis_data, &block_hash)
                };

            match new_chain_state {
                Ok(new_chain_state) => {
                    assert_eq!(new_chain_state.last_block, block_hash);
                    if new_chain_state.chain_length > self.chain_state.chain_length {
                        info!(
                            "switching to new tip {} ({:?}), previous length {}, new length {}",
                            block_hash,
                            new_chain_state.last_date,
                            self.chain_state.chain_length,
                            new_chain_state.chain_length
                        );
                        self.chain_state = new_chain_state;
                        tag::write_hash(&self.storage, &LOCAL_BLOCKCHAIN_TIP_TAG, &block_hash);
                    } else {
                        info!(
                            "discarding shorter incoming fork {} ({:?}, length {}), tip length {}",
                            block_hash,
                            new_chain_state.last_date,
                            new_chain_state.chain_length,
                            self.chain_state.chain_length
                        );
                    }
                }
                Err(err) => error!(
                    "cannot compute chain state for incoming fork {}: {:?}",
                    block_hash, err
                ),
            }
        }

        // Process previously received children of this block.
        if let Some(children) = self.unconnected_blocks.remove(&block_hash) {
            for (child_hash, child_block) in children {
                info!("triggering child block {}", child_hash);
                self.handle_connected_block(child_hash, child_block);
            }
        }
    }

    fn block_exists(&self, block_hash: &BlockHash) -> bool {
        // TODO: we assume as an invariant that if a block exists on
        // disk, its ancestors exist on disk as well. Need to make
        // sure that this invariant is preserved everywhere
        // (e.g. loose block GC should delete blocks in reverse
        // order).
        self.storage
            .block_exists(block_hash.as_hash_bytes())
            .unwrap_or(false)
    }

    /// Request a missing block from the network.
    fn sollicit_block(&mut self, block_hash: &BlockHash) {
        info!("solliciting block {} from the network", block_hash);
        //unimplemented!();
    }
}
*/
