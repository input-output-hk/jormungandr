mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::Block,
    blockchain::{Blockchain, Branch, ErrorKind as BlockchainError, Tip},
    network,
    settings::start::Settings,
};
use chain_storage::{memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;
use slog::Logger;
use std::time::Duration;

pub type NodeStorage = Box<dyn BlockStore<Block = Block> + Send + Sync>;

/// prepare the block storage from the given settings
///
pub fn prepare_storage(setting: &Settings, logger: &Logger) -> Result<NodeStorage, Error> {
    match &setting.storage {
        None => {
            info!(logger, "storing blockchain in memory");
            Ok(Box::new(MemoryBlockStore::new()))
        }
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|err| Error::IO {
                source: err,
                reason: ErrorKind::SQLite,
            })?;
            let mut sqlite = dir.clone();
            sqlite.push("blocks.sqlite");
            info!(logger, "storing blockchain in '{:?}'", sqlite);
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
pub fn prepare_block_0(
    settings: &Settings,
    storage: &NodeStorage,
    logger: &Logger,
) -> Result<Block, Error> {
    use crate::settings::Block0Info;
    match &settings.block_0 {
        Block0Info::Path(path) => {
            use chain_core::property::Deserialize as _;
            debug!(logger, "parsing block0 from file path `{:?}'", path);
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
                debug!(
                    logger,
                    "retrieving block0 from storage with hash {}", block0_id
                );
                let (block0, _block0_info) = storage.get_block(block0_id)?;
                Ok(block0)
            } else {
                debug!(
                    logger,
                    "retrieving block0 from network with hash {}", block0_id
                );
                network::fetch_block(&settings.network, *block0_id, logger).map_err(|e| e.into())
            }
        }
    }
}

pub fn load_blockchain(
    block0: Block,
    storage: NodeStorage,
    block_cache_ttl: Duration,
) -> Result<(Blockchain, Tip), Error> {
    use tokio::prelude::*;

    let mut blockchain = Blockchain::new(storage, block_cache_ttl);

    let main_branch: Branch = match blockchain.load_from_block0(block0.clone()).wait() {
        Err(error) => match error.kind() {
            BlockchainError::Block0AlreadyInStorage => blockchain.load_from_storage(block0).wait(),
            _ => Err(error),
        },
        Ok(branch) => Ok(branch),
    }?;

    Ok((blockchain, Tip::new(main_branch)))
}
