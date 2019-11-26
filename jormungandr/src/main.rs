// Rustc default type_length_limit is too low for complex futures, which generate deeply nested
// monomorphized structured with long signatures. This value is enough for current project.
#![type_length_limit = "10000000"]

extern crate actix_net;
extern crate actix_threadpool;
extern crate actix_web;
extern crate bech32;
extern crate bincode;
extern crate bytes;
extern crate cardano_legacy_address;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate chain_storage;
extern crate chain_storage_sqlite;
extern crate chain_time;
extern crate imhamt;
#[macro_use]
extern crate custom_error;
#[macro_use]
extern crate error_chain;
#[macro_use(try_ready)]
extern crate futures;
extern crate http;
extern crate humantime;
extern crate hyper;
extern crate jormungandr_lib;
#[macro_use]
extern crate lazy_static;
extern crate linked_hash_map;
extern crate native_tls;
extern crate network_core;
extern crate network_grpc;
extern crate poldercast;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use]
extern crate slog;
extern crate juniper;
extern crate slog_async;
#[cfg(feature = "gelf")]
extern crate slog_gelf;
#[cfg(feature = "systemd")]
extern crate slog_journald;
extern crate slog_json;
#[cfg(unix)]
extern crate slog_syslog;
extern crate slog_term;
extern crate structopt;
extern crate thiserror;
extern crate tk_listen;
extern crate tokio;

use crate::{
    blockcfg::{HeaderHash, Leader},
    blockchain::{Blockchain, CandidateForest},
    secure::enclave::Enclave,
    settings::start::Settings,
    utils::{async_msg, task::Services},
};
use futures::Future;
use jormungandr_lib::interfaces::NodeState;
use settings::{start::RawSettings, CommandLine};
use slog::Logger;
use std::time::Duration;

pub mod blockcfg;
pub mod blockchain;
pub mod client;
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
    rest_context: Option<rest::Context>,
    services: Services,
}

const FRAGMENT_TASK_QUEUE_LEN: usize = 1024;
const NETWORK_TASK_QUEUE_LEN: usize = 32;

fn start_services(bootstrapped_node: BootstrappedNode) -> Result<(), start_up::Error> {
    if let Some(context) = bootstrapped_node.rest_context.as_ref() {
        context.set_node_state(NodeState::StartingWorkers)
    }

    let mut services = bootstrapped_node.services;

    // initialize the network propagation channel
    let (network_msgbox, network_queue) = async_msg::channel(NETWORK_TASK_QUEUE_LEN);
    let (fragment_msgbox, fragment_queue) = async_msg::channel(FRAGMENT_TASK_QUEUE_LEN);
    let blockchain_tip = bootstrapped_node.blockchain_tip;
    let blockchain = bootstrapped_node.blockchain;
    let leadership_logs =
        leadership::Logs::new(bootstrapped_node.settings.leadership.log_ttl.into());
    let leadership_garbage_collection_interval =
        bootstrapped_node.settings.leadership.log_ttl.into();

    let stats_counter = StatsCounter::default();

    let (fragment_pool, pool_logs) = {
        let stats_counter = stats_counter.clone();
        let process = fragment::Process::new(
            bootstrapped_node.settings.mempool.fragment_ttl.into(),
            bootstrapped_node.settings.mempool.log_ttl.into(),
            bootstrapped_node
                .settings
                .mempool
                .garbage_collection_interval
                .into(),
            network_msgbox.clone(),
        );

        let pool = process.pool().clone();
        let logs = process.logs().clone();

        services.spawn_future("fragment", move |info| {
            process.start(info, stats_counter, fragment_queue)
        });
        (pool, logs)
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

    let block_task = {
        let mut blockchain = blockchain.clone();
        let mut blockchain_tip = blockchain_tip.clone();
        let mut network_msgbox = network_msgbox.clone();
        let mut fragment_msgbox = fragment_msgbox.clone();
        let mut explorer_msg_box = explorer.as_ref().map(|(msg_box, _context)| msg_box.clone());
        // TODO: we should get this value from the configuration
        let block_cache_ttl: Duration = Duration::from_secs(3600);
        let stats_counter = stats_counter.clone();
        services.spawn_future_with_inputs("block", move |info, input| {
            let candidate_repo = CandidateForest::new(
                blockchain.clone(),
                block_cache_ttl,
                info.logger().new(o!(log::KEY_SUB_TASK => "chain_pull")),
            );
            blockchain::handle_input(
                info,
                &mut blockchain,
                &mut blockchain_tip,
                &candidate_repo,
                &stats_counter,
                &mut network_msgbox,
                &mut fragment_msgbox,
                explorer_msg_box.as_mut(),
                input,
            )
        })
    };

    let client_task = {
        let mut task_data = client::TaskData {
            storage: blockchain.storage().clone(),
            block0_hash: bootstrapped_node.block0_hash,
            blockchain_tip: blockchain_tip.clone(),
        };

        services.spawn_future_with_inputs("client-query", move |info, input| {
            client::handle_input(info, &mut task_data, input)
        })
    };

    {
        let client_msgbox = client_task.clone();
        let fragment_msgbox = fragment_msgbox.clone();
        let block_msgbox = block_task.clone();
        let block0_hash = bootstrapped_node.block0_hash;
        let config = bootstrapped_node.settings.network.clone();
        let channels = network::Channels {
            client_box: client_msgbox,
            transaction_box: fragment_msgbox,
            block_box: block_msgbox,
        };

        services.spawn_future("network", move |info| {
            let params = network::TaskParams {
                config,
                block0_hash,
                input: network_queue,
                channels,
            };
            network::start(info, params)
                // FIXME: more graceful error reporting
                .map_err(|e| panic!(e))
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
    let enclave = Enclave::from_vec(leader_secrets);

    {
        let leadership_logs = leadership_logs.clone();
        let fragment_pool = fragment_pool.clone();
        let block_task = block_task.clone();
        let blockchain_tip = blockchain_tip.clone();
        let enclave = leadership::Enclave::new(enclave.clone());

        services.spawn_future("leadership", move |info| {
            leadership::Module::new(
                info,
                leadership_logs,
                leadership_garbage_collection_interval,
                blockchain_tip,
                fragment_pool,
                enclave,
                block_task,
            )
            .and_then(|module| module.run())
            .map_err(|e| unimplemented!("error in leadership {}", e))
        });
    }

    if let Some(rest_context) = bootstrapped_node.rest_context {
        let full_context = rest::FullContext {
            stats_counter,
            blockchain,
            blockchain_tip: blockchain_tip.clone(),
            network_task: network_msgbox,
            transaction_task: fragment_msgbox,
            logs: pool_logs,
            leadership_logs,
            enclave,
            explorer: explorer.as_ref().map(|(_msg_box, context)| context.clone()),
        };
        rest_context.set_full(full_context);
        rest_context.set_node_state(NodeState::Running);
    };

    {
        let blockchain_tip = blockchain_tip.clone();
        let no_blockchain_updates_warning_interval = bootstrapped_node
            .settings
            .no_blockchain_updates_warning_interval
            .clone();

        services.spawn_future("stuck_notifier", move |info| {
            stuck_notifier::check_last_block_time(
                info,
                blockchain_tip,
                no_blockchain_updates_warning_interval,
            )
        });
    }

    services.wait_any_finished();
    info!(bootstrapped_node.logger, "Shutting down node");
    Ok(())
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
        context.set_node_state(NodeState::Bootstrapping)
    }

    let bootstrap_logger = logger.new(o!(log::KEY_TASK => "bootstrap"));

    let block0_hash = block0.header.hash();

    let block0_explorer = block0.clone();

    // TODO: we should get this value from the configuration
    let block_cache_ttl: Duration = Duration::from_secs(5 * 24 * 3600);

    let (blockchain, blockchain_tip) = start_up::load_blockchain(block0, storage, block_cache_ttl)?;

    let bootstrapped = network::bootstrap(
        &settings.network,
        blockchain.clone(),
        blockchain_tip.clone(),
        &bootstrap_logger,
    )?;

    let explorer_db = if settings.explorer {
        Some(explorer::ExplorerDB::bootstrap(
            block0_explorer,
            &blockchain,
        )?)
    } else {
        None
    };

    if !bootstrapped {
        // TODO, the node didn't manage to connect to any other nodes
        // for the initial bootstrap, that may be an error however
        // it is not necessarily an error, especially in the case the node is
        // the first ever to wake
    }

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
    pub storage: start_up::NodeStorage,
    pub logger: Logger,
    pub rest_context: Option<rest::Context>,
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

    // The log crate is used by some libraries, e.g. tower-grpc.
    // Set up forwarding from log to slog, but only when trace log level is
    // requested, because the logs are very verbose.
    if log_settings
        .0
        .iter()
        .any(|entry| entry.level >= slog::FilterLevel::Trace)
    {
        slog_scope::set_global_logger(logger.new(o!(log::KEY_SCOPE => "global"))).cancel_reset();
        slog_stdlog::init().unwrap();
    }

    let init_logger = logger.new(o!(log::KEY_TASK => "init"));
    info!(init_logger, "Starting {}", env!("FULL_VERSION"),);
    let settings = raw_settings.try_into_settings(&init_logger)?;
    let mut services = Services::new(logger.clone());

    let rest_context = match settings.rest.clone() {
        Some(rest) => {
            let context = rest::Context::new();
            let service_context = context.clone();
            let explorer = settings.explorer;
            let server_handler = rest::start_rest_server(rest, explorer, &context)?;
            services.spawn_future("rest", move |info| {
                service_context.set_logger(info.into_logger());
                server_handler
            });
            Some(context)
        }
        None => None,
    };

    if let Some(context) = rest_context.as_ref() {
        context.set_node_state(NodeState::PreparingStorage)
    }
    let storage = start_up::prepare_storage(&settings, &init_logger)?;

    // TODO: load network module here too (if needed)

    if let Some(context) = rest_context.as_ref() {
        context.set_node_state(NodeState::PreparingBlock0)
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
