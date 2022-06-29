// Rustc default type_length_limit is too low for complex futures, which generate deeply nested
// monomorphized structured with long signatures. This value is enough for current project.
#![type_length_limit = "10000000"]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

use crate::{
    blockcfg::{HeaderHash, Leader},
    blockchain::Blockchain,
    diagnostic::Diagnostic,
    metrics::MetricsBackend,
    secure::enclave::Enclave,
    settings::start::Settings,
    utils::{async_msg, task::Services},
};
use chain_impl_mockchain::leadership::LeadershipConsensus;
use futures::{executor::block_on, prelude::*};
use jormungandr_lib::interfaces::NodeState;
use settings::{start::RawSettings, CommandLine};
use std::{sync::Arc, time::Duration};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tracing::{span, Level, Span};

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod context;
pub mod diagnostic;
pub mod fragment;
pub mod intercom;
pub mod jrpc;
pub mod leadership;
pub mod log;
pub mod metrics;
pub mod network;
pub mod rest;
pub mod secure;
pub mod settings;
pub mod start_up;
pub mod state;
pub mod stuck_notifier;
pub mod topology;
pub mod utils;
pub mod watch_client;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_futures::Instrument;

fn start() -> Result<(), start_up::Error> {
    let initialized_node = initialize_node()?;

    let bootstrapped_node = bootstrap(initialized_node)?;

    start_services(bootstrapped_node)
}

pub struct BootstrappedNode {
    settings: Settings,
    blockchain: Blockchain,
    blockchain_tip: blockchain::Tip,
    block0_hash: HeaderHash,
    context: Option<context::ContextLock>,
    services: Services,
    initial_peers: Vec<topology::Peer>,
    _logger_guards: Vec<WorkerGuard>,
}

const BLOCK_TASK_QUEUE_LEN: usize = 32;
const FRAGMENT_TASK_QUEUE_LEN: usize = 1024;
const NETWORK_TASK_QUEUE_LEN: usize = 64;
const CLIENT_TASK_QUEUE_LEN: usize = 32;
const TOPOLOGY_TASK_QUEUE_LEN: usize = 32;
const WATCH_CLIENT_TASK_QUEUE_LEN: usize = 32;
const BOOTSTRAP_RETRY_WAIT: Duration = Duration::from_secs(5);

fn start_services(bootstrapped_node: BootstrappedNode) -> Result<(), start_up::Error> {
    if let Some(context) = bootstrapped_node.context.as_ref() {
        block_on(async {
            context
                .write()
                .await
                .set_node_state(NodeState::StartingWorkers)
        });
    }

    let mut services = bootstrapped_node.services;

    // initialize the network propagation channel
    let (network_msgbox, network_queue) = async_msg::channel(NETWORK_TASK_QUEUE_LEN);
    let (block_msgbox, block_queue) = async_msg::channel(BLOCK_TASK_QUEUE_LEN);
    let (fragment_msgbox, fragment_queue) = async_msg::channel(FRAGMENT_TASK_QUEUE_LEN);
    let (client_msgbox, client_queue) = async_msg::channel(CLIENT_TASK_QUEUE_LEN);
    let (topology_msgbox, topology_queue) = async_msg::channel(TOPOLOGY_TASK_QUEUE_LEN);
    let blockchain_tip = bootstrapped_node.blockchain_tip;
    let blockchain = bootstrapped_node.blockchain;
    let leadership_logs =
        leadership::Logs::new(bootstrapped_node.settings.leadership.logs_capacity);

    let metrics_builder = crate::metrics::Metrics::builder();

    let simple_metrics_counter = Arc::new(crate::metrics::backends::SimpleCounter::new());
    let metrics_builder = metrics_builder.add_backend(simple_metrics_counter.clone());

    #[cfg(feature = "prometheus-metrics")]
    let (prometheus_metric, metrics_builder) = if bootstrapped_node.settings.prometheus {
        let prometheus = Arc::new(crate::metrics::backends::Prometheus::new());
        (
            Some(prometheus.clone()),
            metrics_builder.add_backend(prometheus),
        )
    } else {
        (None, metrics_builder)
    };

    let stats_counter = metrics_builder.build();

    {
        let block_ref = services.block_on_task("get_tip_block", |_| blockchain_tip.get_ref());
        let block = blockchain.storage().get(block_ref.hash()).unwrap().unwrap();
        stats_counter.set_tip_block(&block, &block_ref);
    }

    let (watch_msgbox, watch_client) = {
        let (msgbox, queue) = async_msg::channel(WATCH_CLIENT_TASK_QUEUE_LEN);

        let blockchain_tip = blockchain_tip.clone();
        let current_tip = block_on(async { blockchain_tip.get_ref().await.header().clone() });

        let (client, message_processor) =
            watch_client::WatchClient::new(current_tip, blockchain.clone());

        services.spawn_future("watch_client", move |info| async move {
            message_processor.start(info, queue).await
        });

        (msgbox, client)
    };

    {
        let blockchain = blockchain.clone();
        let blockchain_tip = blockchain_tip.clone();
        let network_msgbox = network_msgbox.clone();
        let fragment_msgbox = fragment_msgbox.clone();
        // TODO: we should get this value from the configuration
        let block_cache_ttl: Duration = Duration::from_secs(120);
        let stats_counter = stats_counter.clone();
        services.spawn_future("block", move |info| {
            let task_data = blockchain::TaskData {
                blockchain,
                blockchain_tip,
                stats_counter,
                network_msgbox,
                fragment_msgbox,
                watch_msgbox,
                garbage_collection_interval: block_cache_ttl,
            };
            blockchain::start(task_data, info, block_queue)
        });
    }

    {
        let task_data = client::TaskData {
            storage: blockchain.storage().clone(),
            blockchain_tip: blockchain_tip.clone(),
        };

        services.spawn_future("client-query", move |info| {
            client::start(info, task_data, client_queue)
        });
    }

    // FIXME: reduce state sharing across services
    let network_state = Arc::new(network::GlobalState::new(
        bootstrapped_node.block0_hash,
        bootstrapped_node.settings.network.clone(),
        stats_counter.clone(),
        span!(Level::TRACE, "task", kind = "network"),
    ));

    {
        let fragment_msgbox = fragment_msgbox.clone();
        let block_msgbox = block_msgbox.clone();
        let global_state = network_state.clone();
        let channels = network::Channels {
            client_box: client_msgbox,
            transaction_box: fragment_msgbox,
            block_box: block_msgbox,
            topology_box: topology_msgbox.clone(),
        };

        services.spawn_future("network", move |_| {
            let params = network::TaskParams {
                global_state,
                input: network_queue,
                channels,
                watch: watch_client,
            };
            network::start(params)
        });
    }

    {
        let task_data = topology::TaskData {
            config: bootstrapped_node.settings.network.clone(),
            network_msgbox: network_msgbox.clone(),
            initial_peers: bootstrapped_node.initial_peers,
            topology_queue,
            stats_counter: stats_counter.clone(),
        };

        services.spawn_future("topology", move |_| topology::start(task_data));
    }

    let bft_leaders = block_on(async {
        if let LeadershipConsensus::Bft(data) = blockchain_tip
            .get_ref()
            .await
            .epoch_leadership_schedule()
            .consensus()
        {
            Some(
                data.leaders()
                    .iter()
                    .map(|k| k.as_public_key().clone())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    });
    let node_secret = bootstrapped_node
        .settings
        .secret
        .map::<Result<_, start_up::Error>, _>(|secret_path| {
            let secret = secure::NodeSecret::load_from_file(secret_path.as_path())?;
            Ok(secret)
        })
        .transpose()?;

    let leader_secret = node_secret.as_ref().map(|secret| {
        if let (Some(leaders), Some(leader)) = (&bft_leaders, secret.bft()) {
            let public_key = &leader.sig_key.to_public();
            if !leaders.contains(public_key) {
                tracing::warn!(
                    "node was started with a BFT secret key but the corresponding \
                        public key {} is not listed among consensus leaders",
                    public_key
                );
            }
        };
        Leader {
            bft_leader: secret.bft(),
            genesis_leader: secret.genesis(),
        }
    });
    let enclave = Enclave::new(leader_secret);

    #[cfg(feature = "evm")]
    let evm_keys = Arc::new(
        node_secret
            .map(|secret| secret.evm_keys())
            .unwrap_or_default(),
    );

    {
        let logs = leadership_logs.clone();
        let block_message = block_msgbox;
        let tip = blockchain_tip.clone();
        let enclave = leadership::Enclave::new(enclave.clone());
        let pool = fragment_msgbox.clone();
        let rewards_report_all = bootstrapped_node.settings.rewards_report_all;
        let block_hard_deadline = bootstrapped_node.settings.block_hard_deadline;

        services.spawn_try_future("leadership", move |service_info| {
            leadership::Module::new(leadership::ModuleConfig {
                service_info,
                logs,
                tip,
                pool,
                enclave,
                block_message,
                rewards_report_all,
                block_hard_deadline,
            })
            .and_then(|module| module.run())
        });
    }

    {
        let blockchain_tip = blockchain_tip.clone();
        let process = fragment::Process::new(
            bootstrapped_node.settings.mempool.pool_max_entries.into(),
            bootstrapped_node.settings.mempool.log_max_entries.into(),
            network_msgbox.clone(),
        );
        let fragment_log_dir = bootstrapped_node
            .settings
            .mempool
            .persistent_log
            .map(|s| s.dir);

        services.spawn_try_future("fragment", move |info| {
            process.start(
                info,
                stats_counter,
                fragment_queue,
                fragment_log_dir,
                blockchain_tip,
            )
        });
    };

    if let Some(context) = bootstrapped_node.context {
        let full_context = context::FullContext {
            stats_counter: simple_metrics_counter,
            network_task: network_msgbox,
            transaction_task: fragment_msgbox,
            topology_task: topology_msgbox,
            leadership_logs,
            #[cfg(feature = "evm")]
            evm_keys,
            enclave,
            network_state,
            #[cfg(feature = "prometheus-metrics")]
            prometheus: prometheus_metric,
        };
        block_on(async {
            let mut context = context.write().await;
            context.set_full(full_context);
            context.set_node_state(NodeState::Running);
        })
    };

    {
        let blockchain_tip = blockchain_tip;
        let no_blockchain_updates_warning_interval = bootstrapped_node
            .settings
            .no_blockchain_updates_warning_interval;

        services.spawn_future("stuck_notifier", move |_| {
            stuck_notifier::check_last_block_time(
                blockchain_tip,
                no_blockchain_updates_warning_interval,
            )
        });
    }

    match services.wait_any_finished() {
        Ok(()) => {
            tracing::info!("Shutting down node");
            Ok(())
        }
        Err(err) => {
            tracing::error!(
                reason = %err.to_string(),
                "Service has terminated with an error"
            );
            Err(start_up::Error::ServiceTerminatedWithError(err))
        }
    }
}

/// # Bootstrap phase
///
/// done at every startup: we need to bootstrap from whatever local state (including nothing)
/// to the latest network state (or close to latest). until this happen, we don't participate in the network
/// (no block creation) and our network connection(s) is only use to download data.
///
/// Various aspects to do, similar to hermes:
/// * download all the existing blocks
/// * verify all the downloaded blocks
/// * network / peer discoveries (?)
fn bootstrap(initialized_node: InitializedNode) -> Result<BootstrappedNode, start_up::Error> {
    let InitializedNode {
        settings,
        block0,
        storage,
        context,
        mut services,
        cancellation_token,
        _logger_guards,
    } = initialized_node;

    let BootstrapData {
        blockchain,
        blockchain_tip,
        block0_hash,
        context,
        settings,
        initial_peers,
    } = services.block_on_task("bootstrap", |info| {
        bootstrap_internal(
            context,
            info.span().clone(),
            block0,
            storage,
            settings,
            cancellation_token,
        )
    })?;

    Ok(BootstrappedNode {
        settings,
        blockchain,
        blockchain_tip,
        block0_hash,
        context,
        services,
        initial_peers,
        _logger_guards,
    })
}

struct BootstrapData {
    blockchain: Blockchain,
    blockchain_tip: blockchain::Tip,
    block0_hash: HeaderHash,
    context: Option<context::ContextLock>,
    settings: Settings,
    initial_peers: Vec<topology::Peer>,
}

async fn bootstrap_internal(
    context: Option<context::ContextLock>,
    span: Span,
    block0: blockcfg::Block,
    storage: blockchain::Storage,
    settings: Settings,
    cancellation_token: CancellationToken,
) -> Result<BootstrapData, start_up::Error> {
    use futures::future::FutureExt;

    if let Some(context) = context.as_ref() {
        block_on(async {
            context
                .write()
                .await
                .set_node_state(NodeState::Bootstrapping)
        })
    }

    let block0_hash = block0.header().hash();

    let cache_capacity = 102_400;

    let (blockchain, blockchain_tip) =
        start_up::load_blockchain(block0, storage, cache_capacity, settings.rewards_report_all)
            .await?;

    if let Some(context) = &context {
        let mut context = context.write().await;
        context.set_blockchain(blockchain.clone());
        context.set_blockchain_tip(blockchain_tip.clone());
        context.set_bootstrap_stopper(cancellation_token.clone());
    };

    let mut bootstrap_attempt: usize = 0;

    let network_res = loop {
        bootstrap_attempt += 1;

        // If we have exceeded the maximum number of bootstrap attempts, then we break out of the
        // bootstrap loop.
        if let Some(max_bootstrap_attempt) = settings.network.max_bootstrap_attempts {
            if bootstrap_attempt > max_bootstrap_attempt {
                tracing::warn!("maximum allowable bootstrap attempts exceeded, continuing...");
                break None; // maximum bootstrap attempts exceeded, exit loop
            };
        }

        // Will return true if we successfully bootstrap or skip_bootstrap is set.
        let res = network::bootstrap(
            &settings.network,
            blockchain.clone(),
            blockchain_tip.clone(),
            cancellation_token.clone(),
            &span,
        )
        .await?;
        if res.bootstrapped {
            break Some(res); // bootstrap succeeded, exit loop
        }

        tracing::info!(
            "bootstrap attempt #{} failed, trying again in {} seconds...",
            bootstrap_attempt,
            BOOTSTRAP_RETRY_WAIT.as_secs()
        );

        futures::select! {
            _ = tokio::time::sleep(BOOTSTRAP_RETRY_WAIT).fuse() => {},
            _ = cancellation_token.cancelled().fuse() => return Err(start_up::Error::Interrupted),
        }
    };

    if let Some(context) = &context {
        let mut context = context.write().await;
        context.remove_bootstrap_stopper();
    };

    Ok(BootstrapData {
        block0_hash,
        blockchain,
        blockchain_tip,
        context,
        settings,
        initial_peers: network_res
            .map(|res| res.initial_peers)
            .unwrap_or_else(Vec::new),
    })
}

pub struct InitializedNode {
    pub settings: Settings,
    pub block0: blockcfg::Block,
    pub storage: blockchain::Storage,
    pub context: Option<context::ContextLock>,
    pub services: Services,
    pub cancellation_token: CancellationToken,
    pub _logger_guards: Vec<WorkerGuard>,
}

#[cfg(unix)]
fn init_os_signal_watchers(services: &mut Services, token: CancellationToken) {
    use signal::unix::SignalKind;

    let token_1 = token.clone();

    async fn recv_signal_and_cancel(mut signal: signal::unix::Signal, token: CancellationToken) {
        if let Some(()) = signal.recv().await {
            token.cancel();
        }
    }

    services.spawn_future("sigterm_watcher", move |_info| {
        match signal::unix::signal(SignalKind::terminate()) {
            Ok(signal) => recv_signal_and_cancel(signal, token).left_future(),
            Err(e) => {
                tracing::warn!(reason = %e, "failed to install handler for SIGTERM");
                future::pending().right_future()
            }
        }
    });

    services.spawn_future("sigint_watcher", move |_info| {
        match signal::unix::signal(SignalKind::interrupt()) {
            Ok(signal) => recv_signal_and_cancel(signal, token_1).left_future(),
            Err(e) => {
                tracing::warn!(reason = %e, "failed to install handler for SIGINT");
                future::pending().right_future()
            }
        }
    });
}

#[cfg(not(unix))]
fn init_os_signal_watchers(services: &mut Services, token: CancellationToken) {
    use signal::ctrl_c;

    services.spawn_future("ctrl_c_watcher", move |_info| {
        ctrl_c().then(move |result| match result {
            Ok(()) => {
                token.cancel();
                future::ready(()).left_future()
            }
            Err(e) => {
                tracing::warn!(reason = %e, "ctrl+c watcher failed");
                future::pending().right_future()
            }
        })
    });
}

fn initialize_node() -> Result<InitializedNode, start_up::Error> {
    let command_line = CommandLine::load();
    let exit_after_storage_setup = command_line.storage_check;

    if command_line.full_version {
        println!("{}", env!("FULL_VERSION"));
        std::process::exit(0);
    } else if command_line.source_version {
        println!("{}", env!("SOURCE_VERSION"));
        std::process::exit(0);
    }

    let raw_settings = RawSettings::load(command_line)?;

    let log_settings = raw_settings.log_settings();
    let (_logger_guards, log_info_msgs) = log_settings.init_log()?;

    let init_span = span!(Level::TRACE, "task", kind = "init");
    let async_span = init_span.clone();
    let _enter = init_span.enter();
    tracing::info!("Starting {}", env!("FULL_VERSION"),);

    if let Some(msgs) = log_info_msgs {
        // if log settings were overriden, we will have an info
        // message which we can unpack at this point.
        for msg in &msgs {
            tracing::info!("{}", msg);
        }
    }

    let diagnostic = Diagnostic::new()?;
    tracing::debug!("system settings are: {}", diagnostic);

    let settings = raw_settings.try_into_settings()?;

    let storage = start_up::prepare_storage(&settings)?;
    if exit_after_storage_setup {
        tracing::info!("Exiting after successful storage setup");
        std::mem::drop(_enter);
        std::mem::drop(init_span);
        std::mem::drop(storage);
        std::process::exit(0);
    }

    if settings.network.trusted_peers.is_empty() && !settings.network.skip_bootstrap {
        return Err(network::bootstrap::Error::EmptyTrustedPeers.into());
    }

    let mut services = Services::new();

    let cancellation_token = CancellationToken::new();
    init_os_signal_watchers(&mut services, cancellation_token.clone());

    let init_context = |diagnostic| {
        use tokio::sync::RwLock;

        let mut context = context::Context::new();
        context.set_diagnostic_data(diagnostic);
        context.set_node_state(NodeState::PreparingStorage);
        Arc::new(RwLock::new(context))
    };

    let context = match settings.rest.clone() {
        Some(rest_config) => {
            let context = init_context(diagnostic.clone());

            let rest_config = rest::Config {
                listen: rest_config.listen,
                tls: rest_config.tls,
                cors: rest_config.cors,
                #[cfg(feature = "prometheus-metrics")]
                enable_prometheus: settings.prometheus,
            };

            let server_handler = rest::start_rest_server(rest_config, context.clone());
            let service_context = context.clone();
            services.spawn_future("rest", |info| async move {
                service_context.write().await.set_span(info.span().clone());
                server_handler.await
            });
            Some(context)
        }
        None => None,
    };

    #[cfg(feature = "evm")]
    let context = match settings.jrpc.clone() {
        Some(jrpc_config) => {
            let context = context.unwrap_or_else(|| init_context(diagnostic));

            let jrpc_config = jrpc::Config {
                listen: jrpc_config.listen,
            };
            let server_handler = jrpc::start_jrpc_server(jrpc_config, context.clone());
            let service_context = context.clone();
            services.spawn_future("jrpc", |info| async move {
                service_context.write().await.set_span(info.span().clone());
                server_handler.await
            });

            Some(context)
        }
        None => context,
    };

    // TODO: load network module here too (if needed)

    if let Some(context) = context.as_ref() {
        block_on(async {
            context
                .write()
                .await
                .set_node_state(NodeState::PreparingBlock0)
        })
    }

    let block0 = services.block_on_task("prepare_block_0", |_service_info| {
        async {
        use futures::future::FutureExt;

        let cancellation_token = CancellationToken::new();

        if let Some(context) = context.as_ref() {
            let mut context = context.write().await;
            context.set_bootstrap_stopper(cancellation_token.clone());
        }

        let prepare_block0_fut = start_up::prepare_block_0(&settings, &storage).map_err(Into::into);

        let result = futures::select! {
            result = prepare_block0_fut.fuse() => result,
            _ = cancellation_token.cancelled().fuse() => return Err(start_up::Error::Interrupted),
        };

        if let Some(context) = context.as_ref() {
            let mut context = context.write().await;
            context.remove_bootstrap_stopper();
        }

        result
    }.instrument(async_span)
    })?;

    Ok(InitializedNode {
        settings,
        block0,
        storage,
        context,
        services,
        cancellation_token,
        _logger_guards,
    })
}

/// Main entrypoint for the `jormungandr` binary
pub fn main() {
    use std::error::Error;

    if let Err(error) = start() {
        eprintln!("{}", error);
        let mut source = error.source();
        while let Some(err) = source {
            eprintln!(" |-> {}", err);
            source = err.source();
        }

        // TODO: https://github.com/rust-lang/rust/issues/43301
        //
        // as soon as #43301 is stabilized it would be nice to no use
        // `exit` but the more appropriate:
        // https://doc.rust-lang.org/stable/std/process/trait.Termination.html
        std::process::exit(error.code());
    }
}
