use std::sync::{Arc, RwLock};

use cardano_storage::StorageConfig;
use cardano_storage::Storage;

use super::chain_types::ChainTips;
use super::super::blockcfg::{GenesisData, BlockHash};

#[allow(dead_code)]
pub struct Blockchain {
    genesis_hash: BlockHash,
    /// the storage for the overall blockchains (blocks)
    storage: Storage,
    /// possible other known forks
    heads: ChainTips<BlockHash>,
    /// what we think is the real blockchain at this specific moment
    tip: Option<BlockHash>,
}

pub type BlockchainR = Arc<RwLock<Blockchain>>;

impl Blockchain {
    pub fn from_storage(genesis_data: &GenesisData, storage_config: &StorageConfig) -> Self {
        let storage = Storage::init(storage_config).unwrap();
        Blockchain {
            genesis_hash: genesis_data.genesis_prev.clone(),
            storage: storage,
            heads: ChainTips::new(),
            tip: None,
        }
    }

    /// return the latest
    pub fn get_tip(&self) -> BlockHash {
        self.genesis_hash.clone()
    }
}
