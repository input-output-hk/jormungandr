use cardano_storage::StorageConfig;
use cardano_storage::Storage;

/// Contains a locally known fork of the blockchain
pub struct BlockchainFork {
}

#[allow(dead_code)]
pub struct Blockchain {
    /// the storage for the overall blockchains (blocks)
    storage: Storage,
    /// possible other known forks
    heads: Vec<BlockchainFork>,
    /// what we think is the real blockchain at this specific moment
    tip: BlockchainFork,
}

impl Blockchain {
    pub fn from_storage(storage_config: &StorageConfig) -> Self {
        let storage = Storage::init(storage_config).unwrap();
        Blockchain {
            storage: storage,
            heads: Vec::new(),
            tip: BlockchainFork {},
        }
    }
}
