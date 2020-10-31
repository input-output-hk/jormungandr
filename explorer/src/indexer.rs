use crate::db::DB;
use chain_core::property::Header as _;
use chain_impl_mockchain::block::Block;
use chain_ser::deser::Deserialize as _;
use jormungandr_lib::crypto::hash::Hash;
use slog::{info, Logger};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("rest request error")]
    RequestError(#[from] reqwest::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("hash error")]
    HashError(#[from] chain_crypto::hash::Error),
    #[error("url error")]
    UrlError(#[from] url::ParseError),
    #[error(transparent)]
    DBError(#[from] crate::db::error::Error),
}

#[derive(Clone)]
pub struct Indexer {
    pub db: DB,
    rest: RestClient,
    logger: Logger,
}

impl Indexer {
    pub fn new(rest: RestClient, db: crate::db::DB, logger: Logger) -> Self {
        Indexer { db, rest, logger }
    }

    pub async fn apply_or_fetch_block(&mut self, msg: Hash) -> Result<(), IndexerError> {
        info!(self.logger, "applying block {}", msg);
        let mut stack = vec![];
        let mut hash = msg.into_hash();

        loop {
            let block = self.rest.get_block(hash.to_string()).await?;
            hash = block.header.parent_id();

            match self.db.apply_block(block.clone()).await {
                Ok(_gc_root) => break,
                Err(crate::db::error::Error::AncestorNotFound(_missing)) => {
                    stack.push(block);
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        while let Some(block) = stack.pop() {
            self.db.apply_block(block).await.expect("shouldn't fail");
        }

        Ok(())
    }

    pub async fn set_tip(&mut self, tip: Hash) -> Result<(), IndexerError> {
        info!(self.logger, "seting tip to {}", tip);
        self.db.set_tip(tip.into_hash()).await;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RestClient {
    client: reqwest::Client,
    url: url::Url,
}

impl RestClient {
    pub fn new(url: url::Url) -> Self {
        RestClient {
            url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_block(&self, hash: impl AsRef<str>) -> Result<Block, IndexerError> {
        let url = self.url.join("v0/block/")?.join(hash.as_ref())?;
        let block_response = self.client.get(url).send().await?;

        Ok(Block::deserialize(std::io::BufReader::new(
            block_response.bytes().await?.as_ref(),
        ))?)
    }
}
