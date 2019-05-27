#![cfg_attr(feature = "with-bench", feature(test))]
extern crate actix_net;
extern crate actix_web;
extern crate bech32;
extern crate bincode;
extern crate bytes;
extern crate cardano;
extern crate cardano_storage;
extern crate cbor_event;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_crypto;
extern crate chain_impl_mockchain;
extern crate chain_storage;
extern crate chain_storage_sqlite;
extern crate chain_time;
extern crate clap;
extern crate cryptoxide;
extern crate futures;
extern crate generic_array;
extern crate http;
extern crate native_tls;
extern crate network_core;
extern crate network_grpc;
extern crate poldercast;
extern crate rand_chacha;
extern crate tokio;
#[macro_use]
extern crate custom_error;

#[cfg(test)]
extern crate quickcheck;
extern crate rand;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use(o, debug, info, warn, error, crit)]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
extern crate structopt;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

use crate::{
    blockcfg::Leader,
    blockchain::BlockchainR,
    rest::v0::node::stats::StatsCounter,
    settings::start::Settings,
    transaction::TPool,
    utils::{async_msg, task::Services},
};
use chain_impl_mockchain::message::{Message, MessageId};
use futures::Future;
use settings::{start::RawSettings, CommandLine};
use slog::Logger;
use std::sync::{Arc, Mutex, RwLock};

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod intercom;
pub mod leadership;
pub mod log;
pub mod network;
pub mod rest;
pub mod secure;
pub mod settings;
pub mod start_up;
pub mod state;
pub mod transaction;
pub mod utils;

fn start() -> Result<(), start_up::Error> {
    let initialized_node = initialize_node()?;

    let bootstrapped_node = bootstrap(initialized_node)?;

    start_services(bootstrapped_node)
}

pub struct BootstrappedNode {
    settings: Settings,
    blockchain: BlockchainR,
    new_epoch_notifier: tokio::sync::mpsc::Receiver<self::leadership::EpochParameters>,
    logger: Logger,
}

const NETWORK_TASK_QUEUE_LEN: usize = 32;

fn start_services(bootstrapped_node: BootstrappedNode) -> Result<(), start_up::Error> {
    let mut services = Services::new();

    let tpool_data: TPool<MessageId, Message> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    // initialize the network propagation channel
    let (mut network_msgbox, network_queue) = async_msg::channel(NETWORK_TASK_QUEUE_LEN);
    let new_epoch_notifier = bootstrapped_node.new_epoch_notifier;

    let stats_counter = StatsCounter::default();
    let logger = &bootstrapped_node.logger;

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = bootstrapped_node.blockchain.clone();
        let stats_counter = stats_counter.clone();
        services.spawn_with_inputs("transaction", logger, move |info, input| {
            transaction::handle_input(info, &blockchain, &tpool, &stats_counter, input)
        })
    };

    let block_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        let stats_counter = stats_counter.clone();
        services.spawn_future_with_inputs("block", logger, move |info, input| {
            blockchain::handle_input(
                info,
                &blockchain,
                &stats_counter,
                &mut network_msgbox,
                input,
            );
            futures::future::ok(())
        })
    };

    let client_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        services.spawn_with_inputs("client-query", logger, move |info, input| {
            client::handle_input(info, &blockchain, input)
        })
    };

    {
        let client_msgbox = client_task.clone();
        let transaction_msgbox = transaction_task.clone();
        let block_msgbox = block_task.clone();
        let config = bootstrapped_node.settings.network.clone();
        let channels = network::Channels {
            client_box: client_msgbox,
            transaction_box: transaction_msgbox,
            block_box: block_msgbox,
        };

        services.spawn("network", logger, move |info| {
            network::run(config, network_queue, channels, info.into_logger());
        });
    }

    let leader_secrets: Vec<Leader> = bootstrapped_node
        .settings
        .leadership
        .iter()
        .map(|secret_path| {
            let secret = secure::NodeSecret::load_from_file(secret_path.as_path()).unwrap();
            Leader {
                bft_leader: secret.bft(),
                genesis_leader: secret.genesis(),
            }
        })
        .collect();

    if !leader_secrets.is_empty() {
        let tpool = tpool.clone();
        let block_task = block_task.clone();
        let blockchain = bootstrapped_node.blockchain.clone();

        services.spawn_future("leadership", logger, move |info| {
            let process = self::leadership::Process::new(
                info,
                tpool,
                blockchain.lock_read().tip.clone(),
                block_task,
            );

            process.start(leader_secrets, new_epoch_notifier)
        });
    }

    let rest_server = match bootstrapped_node.settings.rest {
        Some(rest) => {
            let context = rest::Context {
                stats_counter,
                blockchain: bootstrapped_node.blockchain.clone(),
                transaction_task: Arc::new(Mutex::new(transaction_task)),
            };
            Some(rest::start_rest_server(&rest, context)?)
        }
        None => None,
    };

    services.wait_all();

    if let Some(server) = rest_server {
        server.stop().wait().unwrap()
    }

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
    } = initialized_node;
    let bootstrap_logger = logger.new(o!(log::KEY_TASK => "bootstrap"));

    let (new_epoch_announcements, new_epoch_notifier) = tokio::sync::mpsc::channel(100);

    let blockchain =
        start_up::load_blockchain(block0, storage, new_epoch_announcements, &bootstrap_logger)?;

    network::bootstrap(&settings.network, blockchain.clone(), &bootstrap_logger);

    Ok(BootstrappedNode {
        settings,
        blockchain,
        new_epoch_notifier,
        logger,
    })
}

pub struct InitializedNode {
    pub settings: Settings,
    pub block0: blockcfg::Block,
    pub storage: start_up::NodeStorage,
    pub logger: Logger,
}

fn initialize_node() -> Result<InitializedNode, start_up::Error> {
    let command_line = CommandLine::load();
    let raw_settings = RawSettings::load(command_line)?;
    let logger = raw_settings.to_logger();

    let init_logger = logger.new(o!(log::KEY_TASK => "init"));
    let settings = raw_settings.try_into_settings(&init_logger)?;
    let storage = start_up::prepare_storage(&settings, &init_logger)?;

    // TODO: load network module here too (if needed)

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
    })
}

fn main() {
    use std::error::Error;

    if let Err(error) = start() {
        eprintln!("{}", error);
        if let Some(source) = error.source() {
            eprintln!("{}", source);
        }

        // TODO: https://github.com/rust-lang/rust/issues/43301
        //
        // as soon as #43301 is stabilized it would be nice to no use
        // `exit` but the more appropriate:
        // https://doc.rust-lang.org/stable/std/process/trait.Termination.html
        std::process::exit(error.code());
    }
}
