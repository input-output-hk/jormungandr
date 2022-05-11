use crate::db::{error::BlockNotFound, ExplorerDb};
use chain_impl_mockchain::block::{Block, HeaderId as HeaderHash};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("could not deserialize response")]
    CannotDeserialize(#[from] serde_json::Error),
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("hash error")]
    HashError(#[from] chain_crypto::hash::Error),
    #[error("url error")]
    UrlError(#[from] url::ParseError),
    #[error(transparent)]
    DbError(#[from] crate::db::error::ExplorerError),
}

#[derive(Clone)]
pub struct Indexer {
    pub db: ExplorerDb,
    tip_candidate: Arc<Mutex<Option<HeaderHash>>>,
}

impl Indexer {
    pub fn new(db: crate::db::ExplorerDb) -> Self {
        let tip_candidate = Arc::new(Mutex::new(None));
        Indexer { db, tip_candidate }
    }

    pub async fn apply_block(&self, block: Block) -> Result<(), IndexerError> {
        tracing::info!("applying {}", block.header().id());

        // TODO: technically this could dispatch a task, as there is a possibility of applying
        // blocks (siblings) in parallel, but that is a mission for another day.  biggest concern
        // is that the we receive two consecutive blocks, if the first is really big and costly to
        // apply, we may try to apply the next one too soon...
        let _state_ref = self.db.apply_block(block.clone()).await?;

        let mut guard = self.tip_candidate.lock().await;
        if guard
            .map(|hash| hash == block.header().id())
            .unwrap_or(false)
        {
            let hash = guard.take().unwrap();
            self.set_tip(hash).await;
        }

        Ok(())
    }

    pub async fn set_tip(&self, tip: HeaderHash) {
        match self.db.set_tip(tip).await {
            Ok(_) => {
                tracing::info!("tip set to {}", tip);
            }
            Err(BlockNotFound { hash: _ }) => {
                // we don't use the value in the error since `tip` is copy anyway
                let mut guard = self.tip_candidate.lock().await;
                guard.replace(tip);
            }
        }
    }
}
