use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use chain_core::property::{self, HasTransaction};
use chain_impl_mockchain::{key, leadership, ledger, setting};
use chain_storage::{error as storage, memory::MemoryBlockStore, store::BlockStore};

use crate::blockcfg::{genesis_data::GenesisData, mock::Mockchain, BlockConfig};
use crate::secure::NodePublic;

/// this structure holds all the state of the blockchains
///
/// It is meant to always be valid.
pub struct State<B: BlockConfig> {
    /// The current chain state corresponding to our tip.
    pub ledger: B::Ledger,
    /// the setting of the blockchain corresponding to out tip
    pub settings: B::Settings,
    pub leaders: B::Leader,
}

pub struct Blockchain<B: BlockConfig> {
    pub genesis_data: B::GenesisData,

    /// the storage for the overall blockchains (blocks)
    pub storage: MemoryBlockStore<B::Block>,

    pub state: State<B>,

    pub change_log: Vec<(
        <B::Leader as property::LeaderSelection>::Update,
        <B::Ledger as property::Ledger>::Update,
        <B::Settings as property::Settings>::Update,
    )>,

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

impl State<Mockchain> {
    pub fn new(genesis: &GenesisData, node_public: Option<NodePublic>) -> Self {
        let last_block_hash = key::Hash::hash_bytes(&[]);

        let leaders = if let Some(public) = node_public {
            leadership::LeaderSelection::BFT(
                leadership::bft::BftLeaderSelection::new(
                    key::PublicKey(public.block_publickey.clone()),
                    genesis.leaders().cloned().collect(),
                )
                .unwrap(),
            )
        } else {
            leadership::LeaderSelection::BFT(
                leadership::bft::BftLeaderSelection::new_passive(
                    genesis.leaders().cloned().collect(),
                )
                .unwrap(),
            )
        };

        State {
            ledger: ledger::Ledger::new(genesis.initial_utxos()),
            settings: setting::Settings {
                last_block_id: last_block_hash,
                max_number_of_transactions_per_block: 100, // TODO: add this in the genesis data ?
            },
            leaders: leaders,
        }
    }
}

impl Blockchain<Mockchain> {
    pub fn new(genesis_data: GenesisData, node_public: Option<NodePublic>) -> Self {
        let last_block_hash = key::Hash::hash_bytes(&[]);

        let state = State::new(&genesis_data, node_public);
        Blockchain {
            genesis_data: genesis_data,
            storage: MemoryBlockStore::new(last_block_hash.clone()),
            state: state,
            change_log: Vec::default(),
            unconnected_blocks: BTreeMap::default(),
        }
    }
}

impl<B: BlockConfig> Blockchain<B>
where
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
    for<'a> &'a <B::Block as property::HasTransaction>::Transactions:
        IntoIterator<Item = &'a B::Transaction>,
{
    pub fn handle_incoming_block(&mut self, block: B::Block) -> Result<(), storage::Error> {
        use chain_core::property::Block;
        let block_hash = block.id();
        let parent_hash = block.parent_id();

        if self.block_exists(&parent_hash)? {
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
        use chain_core::property::{Block, LeaderSelection, Ledger, Settings};

        let current_tip = self.state.settings.tip();

        // Quick optimization: don't do anything if the incoming block
        // is already the tip. Ideally we would bail out if the
        // incoming block is on the tip chain, but there is no quick
        // way to check that.
        if block_hash != current_tip {
            if current_tip == block.parent_id() {
                let leadership_diff = self.state.leaders.diff(&block).unwrap();
                let ledger_diff = { self.state.ledger.diff(block.transactions()).unwrap() };
                let setting_diff = self.state.settings.diff(&block).unwrap();

                self.state.leaders.apply(leadership_diff.clone()).unwrap();
                self.state.ledger.apply(ledger_diff.clone()).unwrap();
                self.state.settings.apply(setting_diff.clone()).unwrap();

                self.change_log
                    .push((leadership_diff, ledger_diff, setting_diff));
                self.storage.put_block(block).unwrap();
                self.storage
                    .put_tag(LOCAL_BLOCKCHAIN_TIP_TAG, &block_hash)
                    .unwrap();
            } else {
                // TODO chain state restoration ?
                unimplemented!()
            }
        }
    }
}
impl<B: BlockConfig> Blockchain<B> {
    fn block_exists(&self, block_hash: &B::BlockHash) -> Result<bool, storage::Error> {
        // TODO: we assume as an invariant that if a block exists on
        // disk, its ancestors exist on disk as well. Need to make
        // sure that this invariant is preserved everywhere
        // (e.g. loose block GC should delete blocks in reverse
        // order).
        self.storage.block_exists(block_hash)
    }

    /// Request a missing block from the network.
    fn sollicit_block(&mut self, block_hash: &B::BlockHash) {
        //unimplemented!();
    }

    /// return the current tip hash and date
    pub fn get_tip(&self) -> B::BlockHash {
        use chain_core::property::Settings;
        self.state.settings.tip()
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
