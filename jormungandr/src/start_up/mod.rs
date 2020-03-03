mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::{Block, HeaderId},
    blockchain::{Blockchain, ErrorKind as BlockchainError, Storage, Tip},
    log, network,
    settings::start::Settings,
};
use chain_storage_sqlite_old::{BlockStore, BlockStoreBuilder, BlockStoreConnection};
use slog::Logger;
use tokio_compat::runtime;

pub type NodeStorage = BlockStore;
pub type NodeStorageConnection = BlockStoreConnection<Block>;

const BLOCKSTORE_BUSY_TIMEOUT: u64 = 1000;

/// prepare the block storage from the given settings
///
pub fn prepare_storage(setting: &Settings, logger: &Logger) -> Result<Storage, Error> {
    let raw_block_store = match &setting.storage {
        None => {
            info!(logger, "storing blockchain in memory");
            BlockStoreBuilder::memory()
                .busy_timeout(BLOCKSTORE_BUSY_TIMEOUT)
                .build()
        }
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|err| Error::IO {
                source: err,
                reason: ErrorKind::SQLite,
            })?;
            let mut sqlite = dir.clone();
            sqlite.push("blocks.sqlite");
            info!(logger, "storing blockchain in '{:?}'", sqlite);
            BlockStoreBuilder::file(sqlite)
                .busy_timeout(BLOCKSTORE_BUSY_TIMEOUT)
                .build()
        }
    };

    Ok(Storage::new(
        raw_block_store,
        logger.new(o!(log::KEY_SUB_TASK => "storage")),
    ))
}

/// Try to fetch the block0_id from the HTTP base URL (services) in the array
///
/// The HTTP url is expecting to be of the form: URL/<hash-id>.block0
async fn fetch_block0_http(
    logger: &Logger,
    base_services: &[String],
    block0_id: &HeaderId,
) -> Option<Block> {
    use chain_core::property::Deserialize as _;

    if base_services.len() == 0 {
        return None;
    }

    async fn fetch_one(block0_id: &HeaderId, url: &str) -> Result<Block, String> {
        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("cannot get {}", e))?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(format!("fetch failed status code: {}", response.status()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("cannot get data {}", e))?;
        let block = Block::deserialize(bytes.as_ref())
            .map_err(|err| format!("parse error on data {}", err))?;
        let got = block.header.id();
        if &got != block0_id {
            return Err(format!("invalid block expecting {} got {}", block0_id, got));
        }
        return Ok(block);
    }

    for base_url in base_services {
        // trying to fetch from service base url
        let url = format!("{}/{}.block0", base_url, block0_id.to_string());
        match fetch_one(block0_id, &url).await {
            Err(e) => {
                debug!(
                    logger,
                    "HTTP fetch : fail to get from {} : error {}", base_url, e
                );
            }
            Ok(block) => {
                info!(logger, "block0 {} fetched by HTTP from {}", block0_id, url);
                return Some(block);
            }
        }
    }

    info!(
        logger,
        "block0 {} fetch by HTTP unsuccesful after trying {} services",
        block0_id,
        base_services.len()
    );
    None
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
    storage: &Storage,
    logger: &Logger,
) -> Result<Block, Error> {
    use crate::settings::Block0Info;
    use chain_core::property::Deserialize as _;
    match &settings.block_0 {
        Block0Info::Path(path, opt_block0_id) => {
            debug!(logger, "parsing block0 from file path `{:?}'", path);
            let f = std::fs::File::open(path).map_err(|err| Error::IO {
                source: err,
                reason: ErrorKind::Block0,
            })?;
            let reader = std::io::BufReader::new(f);
            let block = Block::deserialize(reader).map_err(|err| Error::ParseError {
                source: err,
                reason: ErrorKind::Block0,
            })?;

            // check if the block0 match, the optional expected hash value
            match opt_block0_id {
                None => {}
                Some(expected_hash) => {
                    let got = block.header.id();
                    if &got != expected_hash {
                        return Err(Error::Block0Mismatch {
                            got: got,
                            expected: expected_hash.clone(),
                        });
                    }
                }
            };

            Ok(block)
        }
        Block0Info::Hash(block0_id) => {
            let mut rt = runtime::Builder::new()
                .name_prefix("prepare-block0-worker-")
                .core_threads(1)
                .build()
                .unwrap();

            let storage = storage.back_to_the_future();
            let storage_or_http_block0 = rt.block_on_std(async {
                if let Some(block0) = storage.get(*block0_id).await.unwrap() {
                    debug!(
                        logger,
                        "retrieved block0 from storage with hash {}", block0_id
                    );
                    // TODO verify block0 retrieved is the expected value
                    Some(block0)
                } else {
                    debug!(
                        logger,
                        "retrieving block0 from network with hash {}", block0_id
                    );

                    fetch_block0_http(
                        logger,
                        &settings.network.http_fetch_block0_service,
                        block0_id,
                    )
                    .await
                }
            });
            // fetch from network:: is moved here since it start a runtime, and
            // runtime cannot be started by a runtime.
            match storage_or_http_block0 {
                Some(block0) => Ok(block0),
                None => {
                    let block0 = network::fetch_block(&settings.network, *block0_id, logger)?;
                    Ok(block0)
                }
            }
        }
    }
}

pub fn load_blockchain(
    block0: Block,
    storage: Storage,
    cache_capacity: usize,
    logger: &Logger,
) -> Result<(Blockchain, Tip), Error> {
    let blockchain = Blockchain::new(block0.header.hash(), storage, cache_capacity);

    let mut rt = tokio02::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let main_branch = match blockchain.load_from_block0(block0.clone()).await {
            Err(error) => match error.kind() {
                BlockchainError::Block0AlreadyInStorage => {
                    blockchain.load_from_storage(block0, logger).await
                }
                _ => Err(error),
            },
            Ok(branch) => Ok(branch),
        }?;
        let tip = Tip::new(main_branch);
        let tip_ref = tip.get_ref_std().await;
        info!(
            logger,
            "Loaded from storage tip is : {}",
            tip_ref.header().description()
        );
        Ok((blockchain, tip))
    })
}
