mod api;
pub mod db;
mod indexer;
mod logging;
mod settings;

use crate::indexer::Indexer;
use anyhow::Context;
use chain_core::property::Deserialize;
use chain_impl_mockchain::{block::Block, key::Hash as HeaderHash};
use chain_watch::{
    subscription_service_client::SubscriptionServiceClient, BlockSubscriptionRequest,
    SyncMultiverseRequest, TipSubscriptionRequest,
};
use db::ExplorerDb;
use futures::stream::StreamExt;
use futures_util::{future, pin_mut, stream::FuturesUnordered, FutureExt, TryFutureExt};
use settings::Settings;
use std::convert::TryInto;
use thiserror::Error;
use tokio::{
    select,
    signal::ctrl_c,
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;
use tonic::Streaming;
use tracing::{error, span, Instrument, Level};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IndexerError(#[from] indexer::IndexerError),
    #[error(transparent)]
    SettingsError(#[from] settings::Error),
    #[error(transparent)]
    LoggingError(#[from] logging::Error),
    #[error("failed to bootstrap from node, reason {0}")]
    BootstrapError(#[from] BootstrapError),
    #[error(transparent)]
    Other(anyhow::Error),
    #[error(transparent)]
    UnrecoverableError(anyhow::Error),
}

#[derive(Debug, Error)]
pub enum BootstrapError {
    #[error(transparent)]
    DbError(db::error::ExplorerError),
    #[error("empty bootstrap stream")]
    EmptyStream,
}

#[derive(Clone)]
enum GlobalState {
    Bootstraping,
    Ready(Indexer),
    ShuttingDown,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (_guards, settings) = {
        let mut settings = Settings::load()?;
        let (guards, log_init_messages) = settings.log_settings.take().unwrap().init_log()?;

        let init_span = span!(Level::TRACE, "task", kind = "init");
        let _enter = init_span.enter();
        tracing::info!("Starting explorer");

        if let Some(msgs) = log_init_messages {
            // if log settings were overriden, we will have an info
            // message which we can unpack at this point.
            for msg in &msgs {
                tracing::info!("{}", msg);
            }
        }

        (guards, settings)
    };

    let mut settings = Some(settings);

    let (state_tx, state_rx) = broadcast::channel(3);

    // this unwrap won't panic because the capacity is greater than 1
    state_tx.send(GlobalState::Bootstraping).unwrap();

    let (bootstrap, mut services) = {
        let settings = settings.take().unwrap();

        let mut client = SubscriptionServiceClient::connect(settings.node.clone())
            .await
            .context("Couldn't establish connection with node")
            .map_err(Error::UnrecoverableError)?;

        let sync_stream = client
            .sync_multiverse(SyncMultiverseRequest { from: 0 })
            .await
            .context("Failed to establish bootstrap stream")
            .map_err(Error::UnrecoverableError)?
            .into_inner();

        let block_events = client
            .block_subscription(BlockSubscriptionRequest {})
            .await
            .context("Failed to establish block subscription")
            .map_err(Error::UnrecoverableError)?
            .into_inner();

        let tip_events = client
            .tip_subscription(TipSubscriptionRequest {})
            .await
            .context("Failed to establish tip subscription")
            .map_err(Error::UnrecoverableError)?
            .into_inner();

        let bootstrap = {
            let state_tx = state_tx.clone();

            tokio::spawn(
                async move {
                    let db = bootstrap(sync_stream).await?;

                    let msg = GlobalState::Ready(Indexer::new(db));

                    state_tx
                        .send(msg)
                        .context("failed to broadcast state")
                        .map_err(Error::Other)
                        .map(|_| ())
                }
                .instrument(span!(Level::INFO, "bootstrap task")),
            )
        };

        tracing::info!("starting subscriptions");

        let subscriptions = tokio::spawn(
            process_subscriptions(state_tx.subscribe(), block_events, tip_events)
                .instrument(span!(Level::INFO, "subscriptions")),
        );

        tracing::info!("starting rest task");

        let rest = tokio::spawn(
            async {
                rest_service(state_rx, settings).await;
                Ok(())
            }
            .instrument(span!(Level::INFO, "rest service")),
        );

        (bootstrap, vec![subscriptions, rest])
    };

    let interrupt_handler = tokio::spawn({
        let state_tx = state_tx.clone();

        async move {
            let mut state_rx = state_tx.subscribe();
            let ctrl_c = ctrl_c().fuse();
            pin_mut!(ctrl_c);

            loop {
                select! {
                    s = state_rx.recv() => {
                        if matches!(s.unwrap(), GlobalState::ShuttingDown) {
                            tracing::trace!("shutting down interrupt handler service");
                            break;
                        }
                    }
                    s = (&mut ctrl_c) => {
                        s.context("failed to set interrupt handler")
                            .map_err(Error::UnrecoverableError)?;

                        tracing::trace!("sending ShuttingDown event");

                        state_tx
                            .send(GlobalState::ShuttingDown)
                            .context("failed to send shutdown signal")
                            .map_err(Error::UnrecoverableError)?;

                        break;
                    }
                }
            }

            Ok::<(), Error>(())
        }
    });

    services.push(interrupt_handler);

    let (exit_status, remaining_services) = {
        let bootstrap_status = bootstrap.await;

        if bootstrap_status.is_ok() {
            let (status, _idx, rest) = future::select_all(services).await;
            (status, rest)
        } else {
            (bootstrap_status, services)
        }
    };

    tracing::debug!("sending shutdown event");

    let _ = state_tx.send(GlobalState::ShuttingDown);

    let exit_status = exit_status
        .map_err(|e| Error::UnrecoverableError(e.into()))
        .and_then(std::convert::identity);

    if let Err(error) = exit_status.as_ref() {
        tracing::error!("process finished with error: {:?}", error);

        let _ = future::join_all(remaining_services).await;

        tracing::error!("finished joining on the rest");

        // TODO: map to custom error code
        std::process::exit(1);
    }

    Ok(())
}

async fn bootstrap(mut sync_stream: Streaming<chain_watch::Block>) -> Result<ExplorerDb, Error> {
    tracing::info!("starting bootstrap process");

    let mut db: Option<ExplorerDb> = None;

    // TODO: technically, blocks with the same length can be applied in parallel
    // but it is simpler to do it serially for now at least
    while let Some(block) = sync_stream.next().await {
        let bytes = block
            .context("failed to decode Block received through bootstrap subscription")
            .map_err(Error::UnrecoverableError)?;

        let reader = std::io::BufReader::new(bytes.content.as_slice());

        let block = Block::deserialize(reader)
            .context("failed to decode Block received through bootstrap subscription")
            .map_err(Error::UnrecoverableError)?;

        if let Some(ref db) = db {
            tracing::trace!(
                "applying block {:?} {:?}",
                block.header.hash(),
                block.header.chain_length()
            );
            db.apply_block(block)
                .await
                .map_err(BootstrapError::DbError)?;
        } else {
            db = Some(ExplorerDb::bootstrap(block).map_err(BootstrapError::DbError)?)
        }
    }

    tracing::info!("finish bootstrap process");

    // the only way this unwrap can panic is if we don't get any blocks, I think, not even
    // genesis.. which shouldn't happen
    db.ok_or(BootstrapError::EmptyStream).map_err(Into::into)
}

async fn rest_service(mut state: broadcast::Receiver<GlobalState>, settings: Settings) {
    tracing::info!("starting rest task, waiting for database to be ready");

    let (rest_shutdown, rest_shutdown_signal) = oneshot::channel();
    let (indexer_tx, indexer_rx) = oneshot::channel();

    tokio::spawn(async move {
        let mut indexer_tx = Some(indexer_tx);
        loop {
            match state.recv().await.unwrap() {
                GlobalState::Bootstraping => continue,
                GlobalState::Ready(i) => {
                    if let Some(indexer_tx) = indexer_tx.take() {
                        let _ = indexer_tx.send(i);
                    } else {
                        panic!("received ready event twice");
                    }
                }
                GlobalState::ShuttingDown => {
                    let _ = rest_shutdown.send(());
                    break;
                }
            }
        }
    });

    let db = indexer_rx.await.unwrap().db;

    let api = api::filter(
        db,
        crate::db::Settings {
            address_bech32_prefix: settings.address_bech32_prefix,
        },
    );

    let binding_address = settings.binding_address;
    let tls = settings.tls.clone();
    let cors = settings.cors.clone();

    tracing::info!("starting rest task, listening on {}", binding_address);

    api::setup_cors(api, binding_address, tls, cors, async {
        rest_shutdown_signal.await.unwrap()
    })
    .await;

    tracing::info!("rest task finished");
}

async fn process_subscriptions(
    state: broadcast::Receiver<GlobalState>,
    blocks: Streaming<chain_watch::Block>,
    tips: Streaming<chain_watch::BlockId>,
) -> Result<(), Error> {
    tracing::info!("start consuming subscriptions");

    let blocks = blocks.fuse();
    let tips = tips.fuse();

    let mut indexer = None;

    let (error_tx, mut error_rx) = mpsc::channel(5);
    let (panic_tx, panic_rx) = mpsc::channel(100);

    let status_collector_cancellation_token = CancellationToken::new();

    let task_status_collector = tokio::spawn(
        handle_task_results(
            status_collector_cancellation_token.clone(),
            panic_rx,
            error_tx,
        )
        .instrument(span!(Level::INFO, "handle_task_results")),
    );

    pin_mut!(blocks, tips, task_status_collector, state);

    loop {
        select! {
            state = state.recv() => {
                let state = state.expect("state broadcast channel doesn't have enough capacity");

                tracing::trace!("got state message {:?}", state);

                match state {
                    GlobalState::Bootstraping => continue,
                    GlobalState::Ready(i) => { indexer.replace(i.clone()); },
                    GlobalState::ShuttingDown => {
                        status_collector_cancellation_token.cancel();
                        break;
                    },
                }
            },
            Some(block) = blocks.next() => {
                let indexer = indexer.clone();
                let handle = tokio::spawn(
                    async move {
                        future::ready(block)
                            .map_err(|e| Error::Other(e.into()))
                            .and_then(|block| handle_block(block, indexer))
                            .await
                    }
                    .instrument(span!(Level::INFO, "handle_block")),
                );

                let _ = panic_tx.send(handle).await;
            },
            Some(tip) = tips.next() => {
                tracing::debug!("received tip event");
                let indexer = indexer.clone();

                tokio::spawn(
                    async {
                        handle_tip(
                            tip.context("Failed to receive tip from subscription")
                                .map_err(Error::Other)?,
                            indexer,
                        )
                        .await
                    }
                    .instrument(span!(Level::INFO, "handle_tip")),
                );
            },
            Some(error) = error_rx.recv() => {
                tracing::debug!("received error event");
                return Err(error);
            },
            e = &mut task_status_collector => {
                tracing::debug!("received error from subtask {:?}", e);
                return e
                    .map_err(|e| Error::Other(e.into()))
                    .and_then(std::convert::identity);
            }
            else => break,
        };
    }

    tracing::debug!("finishing subscriptions service");

    task_status_collector
        .await
        .context("task status collector finished with error")
        .map_err(Error::Other)?
}

async fn handle_block(
    raw_block: chain_watch::Block,
    indexer: Option<Indexer>,
) -> Result<(), Error> {
    let reader = std::io::BufReader::new(raw_block.content.as_slice());
    let block = Block::deserialize(reader)
        .context("Failed to deserialize block from block subscription")
        .map_err(Error::Other)?;

    if let Some(indexer) = indexer.as_ref() {
        indexer.apply_block(block).await?;
    } else {
        tracing::trace!(
            "ignoring block {} because database is bootstraping",
            block.header.id()
        );
    }

    Ok(())
}

async fn handle_tip(raw_tip: chain_watch::BlockId, indexer: Option<Indexer>) -> Result<(), Error> {
    let tip: [u8; 32] = raw_tip
        .content
        .as_slice()
        .try_into()
        .context("tip is not 32 bytes long")
        .map_err(Error::Other)?;

    if let Some(indexer) = indexer.as_ref() {
        indexer.set_tip(HeaderHash::from_bytes(tip)).await;
    }

    Ok(())
}

async fn handle_task_results(
    cancellation_token: CancellationToken,
    mut channel: mpsc::Receiver<JoinHandle<Result<(), Error>>>,
    error_tx: mpsc::Sender<Error>,
) -> Result<(), Error> {
    let mut handles = FuturesUnordered::new();

    loop {
        select! {
            _ = cancellation_token.cancelled() => {
                break;
            },
            Some(handle) = channel.recv() => {
                handles.push(handle);
            },
            Some(result) = handles.next() => {
                let error = match result {
                    Ok(Err(err)) => error_tx.send(err).await,
                    Err(err) => error_tx.send(Error::Other(err.into())).await,
                    _ => continue,
                };

                error
                    .context("failed to report task Error to handler")
                    .map_err(Error::Other)?;
            },
            else => break,
        }
    }

    Ok(())
}

// TODO: implement Debug on Indexer so we can derive?
impl std::fmt::Debug for GlobalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalState::Bootstraping => write!(f, "Bootstrapping"),
            GlobalState::Ready(_) => write!(f, "Ready"),
            GlobalState::ShuttingDown => write!(f, "ShuttingDown"),
        }
    }
}
