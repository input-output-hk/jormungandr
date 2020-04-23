// Rustc default type_length_limit is too low for complex futures, which generate deeply nested
// monomorphized structured with long signatures. This value is enough for current project.
#![type_length_limit = "10000000"]

#[macro_use]
extern crate error_chain;
#[macro_use(try_ready)]
extern crate futures;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate slog;
#[cfg(feature = "gelf")]
extern crate slog_gelf;
#[cfg(feature = "systemd")]
extern crate slog_journald;
#[cfg(unix)]
extern crate slog_syslog;

use crate::{
    blockcfg::{HeaderHash, Leader},
    blockchain::Blockchain,
    diagnostic::Diagnostic,
    network::p2p::P2pTopology,
    secure::enclave::Enclave,
    settings::start::Settings,
    utils::{async_msg, task::Services},
};
use futures03::{executor::block_on, future::TryFutureExt};
use jormungandr_lib::interfaces::NodeState;
use settings::{start::RawSettings, CommandLine};
use slog::Logger;
use std::time::Duration;

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod diagnostic;
pub mod explorer;
pub mod fragment;
pub mod intercom;
pub mod leadership;
pub mod log;
pub mod network;
pub mod rest;
pub mod secure;
pub mod settings;
pub mod start_up;
pub mod state;
mod stats_counter;
pub mod stuck_notifier;
pub mod utils;

use stats_counter::StatsCounter;

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
    logger: Logger,
    explorer_db: Option<explorer::ExplorerDB>,
    rest_context: Option<rest::ContextLock>,
    services: Services,
}

const BLOCK_TASK_QUEUE_LEN: usize = 32;
const FRAGMENT_TASK_QUEUE_LEN: usize = 1024;
const NETWORK_TASK_QUEUE_LEN: usize = 32;
const BOOTSTRAP_RETRY_WAIT: Duration = Duration::from_secs(5);

fn start_services(bootstrapped_node: BootstrappedNode) -> Result<(), start_up::Error> {
    if let Some(context) = bootstrapped_node.rest_context.as_ref() {
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
    let blockchain_tip = bootstrapped_node.blockchain_tip;
    let blockchain = bootstrapped_node.blockchain;
    let leadership_logs =
        leadership::Logs::new(bootstrapped_node.settings.leadership.logs_capacity);

    let topology = P2pTopology::new(
        &bootstrapped_node.settings.network,
        bootstrapped_node
            .logger
            .new(o!(log::KEY_TASK => "poldercast")),
    );

    let stats_counter = StatsCounter::default();

    {
        let stats_counter = stats_counter.clone();
        let process = fragment::Process::new(
            bootstrapped_node.settings.mempool.pool_max_entries.into(),
            bootstrapped_node.settings.mempool.log_max_entries.into(),
            network_msgbox.clone(),
        );

        services.spawn_try_future_std("fragment", move |info| {
            process.start(info, stats_counter, fragment_queue)
        });
    };

    let explorer = {
        if bootstrapped_node.settings.explorer {
            let explorer_db = bootstrapped_node
                .explorer_db
                .expect("explorer db to be bootstrapped");

            let mut explorer =
                explorer::Explorer::new(explorer_db.clone(), explorer::graphql::create_schema());

            // Context to give to the rest api
            let context = explorer.clone();

            let task_msg_box = services.spawn_future_with_inputs("explorer", move |info, input| {
                explorer.handle_input(info, input)
            });
            Some((task_msg_box, context))
        } else {
            None
        }
    };

    {
        let blockchain = blockchain.clone();
        let blockchain_tip = blockchain_tip.clone();
        let network_msgbox = network_msgbox.clone();
        let fragment_msgbox = fragment_msgbox.clone();
        let explorer_msgbox = explorer.as_ref().map(|(msg_box, _context)| msg_box.clone());
        // TODO: we should get this value from the configuration
        let block_cache_ttl: Duration = Duration::from_secs(120);
        let stats_counter = stats_counter.clone();
        services.spawn_future_std("block", move |info| {
            let process = blockchain::Process {
                blockchain,
                blockchain_tip,
                stats_counter,
                network_msgbox,
                fragment_msgbox,
                explorer_msgbox,
                garbage_collection_interval: block_cache_ttl,
            };
            process.start(info, block_queue)
        });
    }

    let client_task = {
        let mut task_data = client::TaskData {
            storage: blockchain.storage().clone(),
            blockchain_tip: blockchain_tip.clone(),
            topology: topology.clone(),
        };

        services.spawn_future_with_inputs("client-query", move |info, input| {
            client::handle_input(info, &mut task_data, input)
        })
    };

    {
        let client_msgbox = client_task.clone();
        let fragment_msgbox = fragment_msgbox.clone();
        let block_msgbox = block_msgbox.clone();
        let block0_hash = bootstrapped_node.block0_hash;
        let config = bootstrapped_node.settings.network.clone();
        let stats_counter = stats_counter.clone();
        let channels = network::Channels {
            client_box: client_msgbox,
            transaction_box: fragment_msgbox,
            block_box: block_msgbox,
        };
        let topology = topology.clone();

        services.spawn_future("network", move |info| {
            let params = network::TaskParams {
                config,
                block0_hash,
                input: network_queue,
                channels,
            };
            network::start(info, params, topology, stats_counter)
        });
    }

    let leader_secrets: Result<Vec<Leader>, start_up::Error> = bootstrapped_node
        .settings
        .secrets
        .iter()
        .map(|secret_path| {
            let secret = secure::NodeSecret::load_from_file(secret_path.as_path())?;
            Ok(Leader {
                bft_leader: secret.bft(),
                genesis_leader: secret.genesis(),
            })
        })
        .collect();
    let leader_secrets = leader_secrets?;
    let enclave = block_on(Enclave::from_vec(leader_secrets));

    {
        let leadership_logs = leadership_logs.clone();
        let block_msgbox = block_msgbox.clone();
        let blockchain_tip = blockchain_tip.clone();
        let enclave = leadership::Enclave::new(enclave.clone());
        let fragment_msgbox = fragment_msgbox.clone();

        services.spawn_try_future_std("leadership", move |info| {
            let fut = leadership::Module::new(
                info,
                leadership_logs,
                blockchain_tip,
                fragment_msgbox,
                enclave,
                block_msgbox,
            )
            .and_then(|module| module.run())
            .map_err(|e| {
                eprint!("leadership error: {}", e);
            });

            fut
        });
    }

    if let Some(rest_context) = bootstrapped_node.rest_context {
        let full_context = rest::FullContext {
            stats_counter,
            network_task: network_msgbox,
            transaction_task: fragment_msgbox,
            leadership_logs,
            enclave,
            p2p: topology,
            explorer: explorer.as_ref().map(|(_msg_box, context)| context.clone()),
        };
        block_on(async {
            let mut rest_context = rest_context.write().await;
            rest_context.set_full(full_context);
            rest_context.set_node_state(NodeState::Running);
        })
    };

    {
        let blockchain_tip = blockchain_tip.clone();
        let no_blockchain_updates_warning_interval = bootstrapped_node
            .settings
            .no_blockchain_updates_warning_interval
            .clone();

        services.spawn_future_std("stuck_notifier", move |info| {
            stuck_notifier::check_last_block_time(
                info,
                blockchain_tip,
                no_blockchain_updates_warning_interval,
            )
        });
    }

    match services.wait_any_finished() {
        Err(err) => {
            crit!(
                bootstrapped_node.logger,
                "Service notifier failed to wait for services to shutdown" ;
                "reason" => err.to_string()
            );
            Err(start_up::Error::ServiceTerminatedWithError)
        }
        Ok(true) => {
            info!(bootstrapped_node.logger, "Shutting down node");
            Ok(())
        }
        Ok(false) => {
            crit!(
                bootstrapped_node.logger,
                "Service has terminated with an error"
            );
            Err(start_up::Error::ServiceTerminatedWithError)
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
///
///
fn bootstrap(initialized_node: InitializedNode) -> Result<BootstrappedNode, start_up::Error> {
    let InitializedNode {
        settings,
        block0,
        storage,
        logger,
        rest_context,
        services,
    } = initialized_node;

    if let Some(context) = rest_context.as_ref() {
        block_on(async {
            context
                .write()
                .await
                .set_node_state(NodeState::Bootstrapping)
        })
    }

    let bootstrap_logger = logger.new(o!(log::KEY_TASK => "bootstrap"));

    let block0_hash = block0.header.hash();

    let block0_explorer = block0.clone();

    let cache_capacity = 102_400;

    let (blockchain, blockchain_tip) = start_up::load_blockchain(
        block0,
        storage,
        cache_capacity,
        settings.rewards_report_all,
        &bootstrap_logger,
    )?;

    block_on(async {
        if let Some(rest_context) = &rest_context {
            let mut rest_context = rest_context.write().await;
            rest_context.set_blockchain(blockchain.clone());
            rest_context.set_blockchain_tip(blockchain_tip.clone());
        }
    });

    let mut bootstrap_attempt: usize = 0;
    loop {
        bootstrap_attempt += 1;

        // If we have exceeded the maximum number of bootstrap attempts, then we break out of the
        // bootstrap loop.
        if let Some(max_bootstrap_attempt) = settings.network.max_bootstrap_attempts {
            if bootstrap_attempt > max_bootstrap_attempt {
                warn!(
                    &bootstrap_logger,
                    "maximum allowable bootstrap attempts exceeded, continuing..."
                );
                break; // maximum bootstrap attempts exceeded, exit loop
            };
        }

        // Will return true if we successfully bootstrap or there are no trusted peers defined.
        if network::bootstrap(
            &settings.network,
            blockchain.clone(),
            blockchain_tip.clone(),
            &bootstrap_logger,
        )? {
            break; // bootstrap succeeded, exit loop
        }

        info!(
            &bootstrap_logger,
            "bootstrap attempt #{} failed, trying again in {} seconds...",
            bootstrap_attempt,
            BOOTSTRAP_RETRY_WAIT.as_secs()
        );
        // Sleep for a little while before trying again.
        std::thread::sleep(BOOTSTRAP_RETRY_WAIT);
    }

    let explorer_db = if settings.explorer {
        Some(explorer::ExplorerDB::bootstrap(
            block0_explorer,
            &blockchain,
        )?)
    } else {
        None
    };

    Ok(BootstrappedNode {
        settings,
        block0_hash,
        blockchain,
        blockchain_tip,
        logger,
        explorer_db,
        rest_context,
        services,
    })
}

pub struct InitializedNode {
    pub settings: Settings,
    pub block0: blockcfg::Block,
    pub storage: blockchain::Storage,
    pub logger: Logger,
    pub rest_context: Option<rest::ContextLock>,
    pub services: Services,
}

fn initialize_node() -> Result<InitializedNode, start_up::Error> {
    let command_line = CommandLine::load();

    if command_line.full_version {
        println!("{}", env!("FULL_VERSION"));
        std::process::exit(0);
    } else if command_line.source_version {
        println!("{}", env!("SOURCE_VERSION"));
        std::process::exit(0);
    }

    let raw_settings = RawSettings::load(command_line)?;

    let log_settings = raw_settings.log_settings();
    let logger = log_settings.to_logger()?;

    let init_logger = logger.new(o!(log::KEY_TASK => "init"));
    info!(init_logger, "Starting {}", env!("FULL_VERSION"),);

    let diagnostic = Diagnostic::new()?;
    debug!(init_logger, "system settings are: {}", diagnostic);

    let settings = raw_settings.try_into_settings(&init_logger)?;
    let mut services = Services::new(logger.clone());

    let rest_context = match settings.rest.clone() {
        Some(rest) => {
            use std::sync::Arc;
            use tokio02::sync::RwLock;

            let mut context = rest::Context::new();
            context.set_diagnostic_data(diagnostic);
            context.set_node_state(NodeState::PreparingStorage);
            let context = Arc::new(RwLock::new(context));

            let service_context = context.clone();
            let explorer = settings.explorer;
            let server_handler = rest::start_rest_server(rest, explorer, context.clone());
            services.spawn_future_std("rest", move |info| async move {
                service_context.write().await.set_logger(info.into_logger());
                server_handler.await
            });
            Some(context)
        }
        None => None,
    };

    let storage = start_up::prepare_storage(&settings, &init_logger)?;

    // TODO: load network module here too (if needed)

    if let Some(context) = rest_context.as_ref() {
        block_on(async {
            context
                .write()
                .await
                .set_node_state(NodeState::PreparingBlock0)
        })
    }

    let block0 = start_up::prepare_block_0(
        &settings,
        &storage,
        &init_logger, /* add network to fetch block0 */
    )?;

    Ok(InitializedNode {
        settings,
        block0,
        storage,
        logger,
        rest_context,
        services,
    })
}

fn main() {
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
