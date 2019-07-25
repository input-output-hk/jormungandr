mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::{Block, Block0DataSource as _},
    blockchain::{
        protocols::{Blockchain, Branch, ErrorKind as BlockchainError},
        Blockchain as LegacyBlockchain, BlockchainR as LegacyBlockchainR,
    },
    leadership::{EpochParameters, TaskParameters},
    network,
    settings::start::Settings,
};
use chain_storage::{memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;
use slog::Logger;
use tokio::sync::mpsc;

pub type NodeStorage = Box<BlockStore<Block = Block> + Send + Sync>;

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
                network::fetch_block(&settings.network, &block0_id, logger).map_err(|e| e.into())
            }
        }
    }
}

pub fn load_legacy_blockchain(
    block0: Block,
    storage: NodeStorage,
    epoch_event: mpsc::Sender<EpochParameters>,
    logger: &Logger,
) -> Result<LegacyBlockchainR, Error> {
    let mut blockchain_data = LegacyBlockchain::load(block0, storage, epoch_event, logger)?;
    blockchain_data.initial()?;
    Ok(blockchain_data.into())
}

pub fn load_blockchain(
    block0: Block,
    storage: NodeStorage,
    epoch_event: mpsc::Sender<TaskParameters>,
) -> Result<(Blockchain, Branch), Error> {
    use tokio::prelude::*;

    let start_time = block0.start_time()?;
    let slot_duration = block0.slot_duration()?;

    let time_frame = chain_time::TimeFrame::new(
        chain_time::Timeline::new(start_time),
        chain_time::SlotDuration::from_secs(slot_duration.as_secs() as u32),
    );

    let mut blockchain = Blockchain::new(storage, std::time::Duration::from_secs(3600 * 24 * 30));

    let main_branch: Branch = match blockchain.load_from_block0(block0.clone()).wait() {
        Err(error) => match error.kind() {
            BlockchainError::Block0AlreadyInStorage => blockchain.load_from_storage(block0).wait(),
            _ => Err(error),
        },
        Ok(branch) => Ok(branch),
    }?;

    let blockchain_clone = blockchain.clone();
    main_branch
        .get_ref()
        .and_then(move |reference| blockchain_clone.get_leadership_at_ref(reference))
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |leadership| {
            epoch_event
                .send(TaskParameters {
                    leadership: leadership.clone(),
                    time_frame,
                })
                .into_future()
        })
        .wait()
        .unwrap();

    Ok((blockchain, main_branch))
}
