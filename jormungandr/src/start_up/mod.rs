mod error;

pub use self::error::{Error, ErrorKind};
use crate::{
    blockcfg::{Block, HeaderId},
    blockchain::{Blockchain, Error as BlockchainError, Storage, Tip},
    network,
    settings::start::Settings,
};
use chain_core::packer::Codec;
use tracing::{span, Level};

/// prepare the block storage from the given settings
pub fn prepare_storage(setting: &Settings) -> Result<Storage, Error> {
    let span = span!(Level::TRACE, "sub_task", kind = "storage");
    let storage_span = span.clone();
    let _enter = span.enter();
    if let Some(dir) = &setting.storage {
        std::fs::create_dir_all(dir).map_err(|err| Error::Io {
            source: err,
            reason: ErrorKind::BlockStorage,
        })?;

        tracing::info!("storing blockchain in '{:?}'", dir);

        Storage::file(dir, storage_span).map_err(Into::into)
    } else {
        Storage::memory(storage_span).map_err(Into::into)
    }
}

/// Try to fetch the block0_id from the HTTP base URL (services) in the array
///
/// The HTTP url is expecting to be of the form: URL/<hash-id>.block0
async fn fetch_block0_http(base_services: &[String], block0_id: &HeaderId) -> Option<Block> {
    use chain_core::property::Deserialize as _;

    if base_services.is_empty() {
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
        let block = Block::deserialize(&mut Codec::new(bytes.as_ref()))
            .map_err(|err| format!("parse error on data {}", err))?;
        let got = block.header().id();
        if &got != block0_id {
            return Err(format!("invalid block expecting {} got {}", block0_id, got));
        }
        Ok(block)
    }

    for base_url in base_services {
        // trying to fetch from service base url
        let url = format!("{}/{}.block0", base_url, block0_id);
        match fetch_one(block0_id, &url).await {
            Err(e) => {
                tracing::debug!("HTTP fetch : fail to get from {} : error {}", base_url, e);
            }
            Ok(block) => {
                tracing::info!("block0 {} fetched by HTTP from {}", block0_id, url);
                return Some(block);
            }
        }
    }

    tracing::info!(
        "block0 {} fetch by HTTP unsuccessful after trying {} services",
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
pub async fn prepare_block_0(settings: &Settings, storage: &Storage) -> Result<Block, Error> {
    use crate::settings::Block0Info;
    use chain_core::property::Deserialize as _;
    match &settings.block_0 {
        Block0Info::Path(path, opt_block0_id) => {
            tracing::debug!("parsing block0 from file path `{:?}'", path);
            let f = std::fs::File::open(path).map_err(|err| Error::Io {
                source: err,
                reason: ErrorKind::Block0,
            })?;
            let reader = std::io::BufReader::new(f);
            let block =
                Block::deserialize(&mut Codec::new(reader)).map_err(|err| Error::ParseError {
                    source: err,
                    reason: ErrorKind::Block0,
                })?;

            // check if the block0 match, the optional expected hash value
            match opt_block0_id {
                None => {}
                Some(expected_hash) => {
                    let got = block.header().id();
                    if &got != expected_hash {
                        return Err(Error::Block0Mismatch {
                            got,
                            expected: *expected_hash,
                        });
                    }
                }
            };

            Ok(block)
        }
        Block0Info::Hash(block0_id) => {
            let storage_or_http_block0 = {
                if let Some(block0) = storage.get(*block0_id).unwrap() {
                    tracing::debug!("retrieved block0 from storage with hash {}", block0_id);
                    // TODO verify block0 retrieved is the expected value
                    Some(block0)
                } else {
                    tracing::debug!("retrieving block0 from network with hash {}", block0_id);

                    fetch_block0_http(&settings.network.http_fetch_block0_service, block0_id).await
                }
            };
            // fetch from network:: is moved here since it start a runtime, and
            // runtime cannot be started by a runtime.
            match storage_or_http_block0 {
                Some(block0) => Ok(block0),
                None => {
                    let block0 = network::fetch_block(&settings.network, *block0_id).await?;
                    Ok(block0)
                }
            }
        }
    }
}

pub async fn load_blockchain(
    block0: Block,
    storage: Storage,
    cache_capacity: usize,
    rewards_report_all: bool,
) -> Result<(Blockchain, Tip), Error> {
    let blockchain = Blockchain::new(
        block0.header().hash(),
        storage,
        cache_capacity,
        rewards_report_all,
    );

    let tip = match blockchain.load_from_block0(block0.clone()).await {
        Err(error) => match error {
            BlockchainError::Block0AlreadyInStorage => blockchain.load_from_storage(block0).await,
            error => Err(error),
        },
        Ok(branch) => Ok(branch),
    }
    .map_err(Box::new)?;
    let tip_ref = tip.get_ref().await;
    tracing::info!(
        "Loaded from storage tip is : {}",
        tip_ref.header().description()
    );
    Ok((blockchain, tip))
}
