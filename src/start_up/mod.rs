mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::Block,
    blockchain::{Blockchain, BlockchainR},
    leadership::EpochParameters,
    network,
    settings::start::Settings,
};
use chain_storage::{memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;
use tokio::sync::mpsc;

pub type NodeStorage = Box<BlockStore<Block = Block> + Send + Sync>;

/// prepare the block storage from the given settings
///
pub fn prepare_storage(setting: &Settings) -> Result<NodeStorage, Error> {
    match &setting.storage {
        None => {
            info!("storing blockchain in memory");
            Ok(Box::new(MemoryBlockStore::new()))
        }
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|err| Error::IO {
                source: err,
                reason: ErrorKind::SQLite,
            })?;
            let mut sqlite = dir.clone();
            sqlite.push("blocks.sqlite");
            info!("storing blockchain in '{:?}'", sqlite);
            Ok(Box::new(SQLiteBlockStore::new(sqlite)))
        }
    }
}

/// loading the block 0 is not as trivial as it seems,
/// there are different cases that we may encounter:
///
/// 1. we have the block_0 given as parameter of the settings: easy, we read it;
/// 2. we have the block_0 hash only:
///     1. check the storage if we don't have it already there;
///     2. check the network nodes we know about
pub fn prepare_block_0(settings: &Settings, storage: &NodeStorage) -> Result<Block, Error> {
    use crate::settings::Block0Info;
    match &settings.block_0 {
        Block0Info::Path(path) => {
            use chain_core::property::Deserialize as _;
            debug!("parsing block0 from file path `{:?}'", path);
            let f = std::fs::File::open(path).map_err(|err| Error::IO {
                source: err,
                reason: ErrorKind::Block0,
            })?;
            let reader = std::io::BufReader::new(f);
            Block::deserialize(reader).map_err(|err| Error::ParseError {
                source: err,
                reason: ErrorKind::Block0,
            })
        }
        Block0Info::Hash(block0_id) => {
            if storage.block_exists(&block0_id)? {
                debug!("retrieving block0 from storage with hash {}", block0_id);
                let (block0, _block0_info) = storage.get_block(block0_id)?;
                Ok(block0)
            } else {
                debug!("retrieving block0 from network with hash {}", block0_id);
                network::fetch_block(&settings.network, &block0_id).map_err(|e| e.into())
            }
        }
    }
}

pub fn load_blockchain(
    block0: Block,
    storage: NodeStorage,
    epoch_event: mpsc::Sender<EpochParameters>,
) -> Result<BlockchainR, Error> {
    let mut blockchain_data = Blockchain::load(block0, storage, epoch_event)?;
    blockchain_data.initial()?;
    Ok(blockchain_data.into())
}
