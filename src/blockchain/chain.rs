use std::sync::{Arc, RwLock};

use cardano::block;
use cardano_storage::StorageConfig;
use cardano_storage::Storage;

use super::chain_types::ChainTips;

#[allow(dead_code)]
pub struct Blockchain {
    /// the storage for the overall blockchains (blocks)
    storage: Storage,
    /// possible other known forks
    heads: ChainTips<block::HeaderHash>,
    /// what we think is the real blockchain at this specific moment
    tip: Option<block::HeaderHash>,
}

pub type BlockchainR = Arc<RwLock<Blockchain>>;

impl Blockchain {
    pub fn from_storage(storage_config: &StorageConfig) -> Self {
        let storage = Storage::init(storage_config).unwrap();
        Blockchain {
            storage: storage,
            heads: ChainTips::new(),
            tip: None,
        }
    }
}
