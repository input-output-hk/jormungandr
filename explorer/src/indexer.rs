use crate::db::ExplorerDb;
use chain_impl_mockchain::block::Block;
use chain_impl_mockchain::block::HeaderId as HeaderHash;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{span, Instrument, Level};

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
    DbError(#[from] crate::db::error::DbError),
}

#[derive(Clone)]
pub struct Indexer {
    pub db: ExplorerDb,
    pub tip_broadcast: tokio::sync::broadcast::Sender<HeaderHash>,
    tip_candidate: Arc<Mutex<Option<HeaderHash>>>,
}

impl Indexer {
    pub fn new(
        db: crate::db::ExplorerDb,
        tip_broadcast: tokio::sync::broadcast::Sender<HeaderHash>,
    ) -> Self {
        let tip_candidate = Arc::new(Mutex::new(None));
        Indexer {
            db,
            tip_broadcast,
            tip_candidate,
        }
    }

    pub async fn apply_block(&self, block: Block) -> Result<(), IndexerError> {
        let span = span!(Level::INFO, "Indexer::apply_block");

        async move {
            tracing::info!(
                "applying {} {}",
                block.header.id(),
                block.header.chain_length()
            );

            self.db.apply_block(block.clone()).await?;

            let mut guard = self.tip_candidate.lock().await;
            if guard.map(|hash| hash == block.header.id()).unwrap_or(false) {
                let hash = guard.take().unwrap();
                self.set_tip(hash).await?;
            }

            Ok(())
        }
        .instrument(span)
        .await
    }

    pub async fn set_tip(&self, tip: HeaderHash) -> Result<(), IndexerError> {
        let span = span!(Level::INFO, "Indexer::set_tip");

        async move {
            let successful = self.db.set_tip(tip).await?;

            if !successful {
                let mut guard = self.tip_candidate.lock().await;
                guard.replace(tip);
            } else {
                tracing::info!("tip set to {}", tip);

                if let Err(e) = self.tip_broadcast.send(tip) {
                    tracing::warn!(?e);
                }
            }

            Ok(())
        }
        .instrument(span)
        .await
    }
}
