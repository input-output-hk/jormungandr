mod api;
mod db;
mod indexer;
mod settings;
mod subscription;

use crate::indexer::Indexer;
use api::graphql::GraphQLSettings;
use backoff::{future::FutureOperation as _, ExponentialBackoff};
use db::DB;
use futures_util::{future::BoxFuture, pin_mut, FutureExt, TryStreamExt};
use settings::Settings;
use slog::{error, info, trace, Logger};
use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;
use std::sync::Arc;
use subscription::SubscriptionError;
use thiserror::Error;
use tokio::{signal::ctrl_c, sync::RwLock};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IndexerError(#[from] indexer::IndexerError),
    #[error(transparent)]
    SubscriptionError(#[from] subscription::SubscriptionError),
}

#[tokio::main]
async fn main() {
    let logger = init_logger();
    let settings = Settings::load().unwrap();

    let notifier_url = settings.notifier_url();
    let rest_url = settings.rest_url();

    let rest = indexer::RestClient::new(rest_url);

    info!(logger, "fetching block0 from rest");

    let block0 = match rest.get_block(&settings.block0_hash).await {
        Ok(block) => block,
        Err(err) => {
            error!(logger, "failed to fetch block0: {}", err);
            return;
        }
    };

    let db = match DB::bootstrap(block0).await {
        Ok(db) => db,
        Err(err) => {
            error!(logger, "failed to bootstrap database: {}", err);
            return;
        }
    };

    let indexer = Arc::new(RwLock::new(Indexer::new(rest, db.clone(), logger.clone())));

    let api = api::filter(
        db,
        GraphQLSettings {
            address_bech32_prefix: settings.address_bech32_prefix.clone(),
        },
    );

    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut tx = Some(tx);

    let mut shutdown_signal = {
        let logger = logger.clone();
        tokio::spawn(async move {
            ctrl_c().await.expect("Error setting Ctrl-C handler");
            if let Err(error) = tx.take().unwrap().send(()) {
                error!(logger, "failed to send shutdown signal {:?}", error);
            }
        })
    }
    .fuse();

    tokio::spawn((|| {
        let binding_address = settings.binding_address.clone();
        let tls = settings.tls.clone();
        let cors = settings.cors.clone();
        async move {
            api::setup_cors(api, binding_address, tls, cors, async move {
                rx.await.ok();
            })
            .await
        }
    })());

    let subscription =
        start_subscription_with_retry(logger.clone(), notifier_url, Arc::clone(&indexer)).fuse();

    pin_mut!(subscription);

    info!(logger, "starting subscription");
    futures::select! {
        _ = shutdown_signal => {
            info!(logger, "interruption signal received, stopping service");
            return; },
        s = subscription => {
            if let Err(e) = s {
                error!(logger, "failed to start subscription: {}", e);
            }
        }
    }
}

fn init_logger() -> Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stderr);
    builder.build().expect("Couldn't initialize logger")
}

type SharedIndexer = Arc<RwLock<Indexer>>;

async fn start_subscription_with_retry(
    logger: Logger,
    notifier_url: url::Url,
    indexer: SharedIndexer,
) -> Result<(), Error> {
    let op = move || -> BoxFuture<Result<(), backoff::Error<Error>>> {
        let notifier_url = notifier_url.clone();
        let indexer = Arc::clone(&indexer);
        let logger = logger.clone();

        Box::pin(async move {
            let indexer = Arc::clone(&indexer);

            let sub = subscription::start_subscription(notifier_url.clone(), logger.clone())
                .await
                .map_err(|err| match dbg!(err) {
                    e if matches!(
                        &e,
                        subscription::SubscriptionError::Tungstenite(
                            async_tungstenite::tungstenite::Error::Io(_),
                        )
                    ) =>
                    {
                        error!(
                            logger,
                            "failed to establish connection with node, reason {}", &e
                        );
                        backoff::Error::Transient(Error::from(e))
                    }
                    e => backoff::Error::Permanent(Error::from(e)),
                })?;

            sub.map_err(Error::from)
                .try_for_each(|msg| {
                    let indexer = Arc::clone(&indexer);
                    let logger = logger.clone();

                    async move {
                        trace!(logger, "processing new subscription message {:?}", &msg);
                        match msg {
                            subscription::JsonMessage::NewBlock(hash) => {
                                indexer.write().await.apply_or_fetch_block(hash).await?;
                            }
                            subscription::JsonMessage::NewTip(hash) => {
                                indexer.write().await.set_tip(hash).await?;
                            }
                        }
                        Ok(())
                    }
                })
                .await
                .map_err(|err| match err {
                    e if matches!(
                        e,
                        Error::SubscriptionError(SubscriptionError::MaxConnectionsReached) |
                        Error::SubscriptionError(SubscriptionError::Tungstenite(
                           async_tungstenite::tungstenite::Error::Protocol(_)
                        ))
                    ) =>
                    {
                        error!(logger, "couldn't connect to notifier");
                        backoff::Error::Transient(e)
                    }
                    e => backoff::Error::Permanent(e),
                })
        })
    };

    op.retry(ExponentialBackoff::default()).await
}
