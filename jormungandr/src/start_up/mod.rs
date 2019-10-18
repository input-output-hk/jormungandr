mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::{Block, Leadership},
    blockchain::{Blockchain, Branch, ErrorKind as BlockchainError, Tip},
    leadership::NewEpochToSchedule,
    network,
    settings::start::Settings,
};
use chain_storage::{memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;
use slog::Logger;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc;

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
    epoch_event: mpsc::Sender<NewEpochToSchedule>,
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

    main_branch
        .get_ref()
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |reference| {
            let time_frame = reference.time_frame();
            let current_known_leadership = reference.epoch_leadership_schedule();
            let current_known_state = reference.ledger();

            let slot = time_frame
                .slot_at(&std::time::SystemTime::now())
                .ok_or(Error::Block0InFuture)
                .unwrap();
            let date = current_known_leadership
                .era()
                .from_slot_to_era(slot)
                .unwrap();
            let new_schedule = Leadership::new(date.epoch.0, &current_known_state);

            epoch_event
                .send(NewEpochToSchedule {
                    new_schedule: Arc::new(new_schedule),
                    new_parameters: reference.epoch_ledger_parameters().clone(),
                    time_frame: reference.time_frame().as_ref().clone(),
                })
                .into_future()
        })
        .wait()
        .unwrap();

    Ok((blockchain, Tip::new(main_branch)))
}
