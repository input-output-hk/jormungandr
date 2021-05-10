mod api;
mod db;
mod indexer;
mod settings;

use crate::indexer::Indexer;
use chain_core::property::Deserialize;
use chain_impl_mockchain::{block::Block, key::Hash as HeaderHash};
use chain_watch::{
    subscription_service_client::SubscriptionServiceClient, BlockSubscriptionRequest,
    SyncMultiverseRequest, TipSubscriptionRequest,
};
use db::ExplorerDb;
use futures::stream::StreamExt;
use futures_util::FutureExt;
use settings::Settings;
use std::convert::TryInto;
use std::sync::Arc;
use thiserror::Error;
use tokio::{signal::ctrl_c, sync::watch};
use tonic::Streaming;
use tracing::{error, span, Instrument, Level};
use tracing_subscriber::FmtSubscriber;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IndexerError(#[from] indexer::IndexerError),
    #[error(transparent)]
    SettingsError(#[from] settings::Error),
}

#[derive(Clone)]
enum GlobalState {
    Bootstraping,
    Ready(Indexer),
    ShuttingDown,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    let settings = Settings::load()?;

    let mut bootstrap_client = SubscriptionServiceClient::connect(settings.node.clone())
        .await
        .unwrap();

    let sync_stream = bootstrap_client
        .sync_multiverse(SyncMultiverseRequest { from: 0 })
        .await
        .unwrap()
        .into_inner();

    let mut settings = Some(settings);

    let (state_tx, state_rx) = watch::channel(GlobalState::Bootstraping);
    let state_tx = Arc::new(state_tx);

    let ready_tx = Arc::clone(&state_tx);

    tokio::spawn(
        async move {
            let db = bootstrap(sync_stream).await;
            tracing::debug!("sending ready event");
            if let Err(_) = ready_tx.send(GlobalState::Ready(Indexer::new(db))) {
                todo!("failed to broadcast Ready state after bootstrapping");
            }
        }
        .instrument(span!(Level::INFO, "bootstrap task")),
    );

    let tasks = async move {
        let mut state_rx = state_rx;
        while state_rx.changed().await.is_ok() {
            let state = state_rx.borrow().clone();
            match state {
                GlobalState::Bootstraping => (),
                GlobalState::Ready(indexer) => {
                    let settings = settings.take().unwrap();
                    let mut client = SubscriptionServiceClient::connect(settings.node.clone())
                        .await
                        .unwrap();

                    tracing::info!("database ready, start processing live feed");
                    let block_events = client
                        .block_subscription(BlockSubscriptionRequest {})
                        .await
                        .unwrap()
                        .into_inner();

                    let tip_events = client
                        .tip_subscription(TipSubscriptionRequest {})
                        .await
                        .unwrap()
                        .into_inner();

                    tokio::spawn(
                        block_subscription(indexer.clone(), block_events)
                            .instrument(span!(Level::INFO, "block subscription task")),
                    );

                    tokio::spawn(
                        tip_subscription(indexer.clone(), tip_events)
                            .instrument(span!(Level::INFO, "tip subscription")),
                    );

                    tokio::spawn(
                        rest_service(indexer.db.clone(), settings)
                            .instrument(span!(Level::INFO, "rest service")),
                    );
                }
                GlobalState::ShuttingDown => {
                    return;
                }
            }
        }
    };

    tokio::spawn(tasks);

    ctrl_c().await.expect("Error setting Ctrl-C handler");

    if let Err(_error) = state_tx.send(GlobalState::ShuttingDown) {
        tracing::warn!("failed to send shutdown signal");
    }

    Ok(())
}

async fn bootstrap(mut sync_stream: Streaming<chain_watch::Block>) -> ExplorerDb {
    tracing::info!("starting bootstrap process");

    let mut db: Option<ExplorerDb> = None;

    // TODO: technically, blocks with the same length can be applied in parallel
    // but it is simpler to do it serially for now at least
    while let Some(block) = sync_stream.next().await {
        let bytes = block.unwrap();
        let reader = std::io::BufReader::new(bytes.content.as_slice());
        let block = Block::deserialize(reader).unwrap();

        if let Some(ref db) = db {
            tracing::trace!(
                "applying block {:?} {:?}",
                block.header.hash(),
                block.header.chain_length()
            );
            db.apply_block(block).await.unwrap();
        } else {
            db = Some(ExplorerDb::bootstrap(block).unwrap())
        }
    }

    tracing::info!("end bootstrap process");

    db.unwrap()
}

async fn rest_service(db: ExplorerDb, settings: Settings) {
    tracing::info!("starting rest task");
    let api = api::filter(
        db,
        crate::db::Settings {
            address_bech32_prefix: settings.address_bech32_prefix,
        },
    );

    let binding_address = settings.binding_address.clone();
    let tls = settings.tls.clone();
    let cors = settings.cors.clone();

    api::setup_cors(
        api,
        binding_address,
        tls,
        cors,
        tokio::signal::ctrl_c().map(|_| ()),
    )
    .await;
}

async fn block_subscription(
    indexer: Indexer,
    mut blocks: Streaming<chain_watch::Block>,
) -> Result<(), Error> {
    tracing::info!("starting block subscription");
    while let Some(block) = blocks.next().await {
        let bytes = block.unwrap();
        let reader = std::io::BufReader::new(bytes.content.as_slice());
        let block = Block::deserialize(reader).unwrap();
        indexer.apply_block(block).await?;
    }

    Ok(())
}

async fn tip_subscription(indexer: Indexer, mut tips: Streaming<chain_watch::BlockId>) {
    tracing::info!("starting tip subscription");
    while let Some(tip) = tips.next().await {
        let tip: [u8; 32] = tip.unwrap().content.as_slice().try_into().unwrap();
        indexer.set_tip(HeaderHash::from_bytes(tip)).await;
    }
}
