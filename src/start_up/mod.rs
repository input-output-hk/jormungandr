mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::{Block, Block0DataSource as _},
    blockchain::{Blockchain, BlockchainR},
    clock::{Clock, ClockEpochConfiguration},
    settings::{logging::LogSettings, start::Settings, CommandLine},
};
use chain_storage::{memory::MemoryBlockStore, store::BlockStore};
use chain_storage_sqlite::SQLiteBlockStore;

pub type NodeStorage = Box<BlockStore<Block = Block> + Send + Sync>;

/// this function prepare the resources of the application
///
/// 1. prepare the default logger
///
pub fn prepare_resources() -> Result<(), Error> {
    // prepare initial logger
    LogSettings::default().apply();

    Ok(())
}

pub fn load_command_line() -> Result<CommandLine, Error> {
    Ok(CommandLine::load())
}

pub fn load_settings(command_line: &CommandLine) -> Result<Settings, Error> {
    Ok(Settings::load(command_line)?)
}

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

/// prepare the logger
pub fn prepare_logger(settings: &Settings) -> Result<(), Error> {
    settings.log_settings.apply();
    Ok(())
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

                unimplemented!("Retrieving the block0 from network")
            }
        }
    }
}

pub fn prepare_clock(block0: &Block) -> Result<Clock, Error> {
    let start_time = block0.start_time()?;
    let slot_duration = block0.slot_duration()?;
    let slots_per_epoch = block0.slots_per_epoch()?;

    let initial_epoch = ClockEpochConfiguration {
        slot_duration,
        slots_per_epoch: slots_per_epoch.unwrap_or(10 * 10),
    };

    info!(
        "blockchain started the {} ({})",
        humantime::format_rfc3339(start_time),
        humantime::format_duration(
            start_time
                .elapsed()
                .expect("start time must be set in the past")
        ),
    );

    Ok(Clock::new(start_time, initial_epoch))
}

pub fn load_blockchain(block0: Block, storage: NodeStorage) -> Result<BlockchainR, Error> {
    let blockchain_data = Blockchain::load(block0, storage)?;
    Ok(blockchain_data.into())
}
