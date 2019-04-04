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
extern crate clap;
extern crate cryptoxide;
extern crate futures;
extern crate generic_array;
extern crate http;
#[macro_use]
extern crate lazy_static;
extern crate native_tls;
extern crate network_core;
extern crate network_grpc;
extern crate poldercast;
extern crate rand_chacha;
extern crate tower_service;
#[macro_use]
extern crate custom_error;

extern crate tokio;

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
#[macro_use(o, slog_trace, slog_debug, slog_info, slog_warn, slog_error, slog_crit)]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
extern crate structopt;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

use std::sync::{Arc, Mutex, RwLock};

use futures::Future;

use chain_impl_mockchain::message::{Message, MessageId};

use crate::{
    blockcfg::{BftLeader, Leader},
    blockchain::BlockchainR,
    intercom::BlockMsg,
    leadership::leadership_task,
    rest::v0::node::stats::StatsCounter,
    settings::start::Settings,
    transaction::TPool,
    utils::{async_msg, task::Services},
};

#[macro_use]
pub mod log_wrapper;

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod clock;
// pub mod consensus;
pub mod intercom;
pub mod leadership;
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

    start_services(&bootstrapped_node)
}

pub struct BootstrappedNode {
    settings: Settings,
    clock: clock::Clock,
    blockchain: BlockchainR,
}

const NETWORK_TASK_QUEUE_LEN: usize = 32;

fn start_services(bootstrapped_node: &BootstrappedNode) -> Result<(), start_up::Error> {
    let mut services = Services::new();

    let tpool_data: TPool<MessageId, Message> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    // initialize the network propagation channel
    let (network_msgbox, network_queue) = async_msg::channel(NETWORK_TASK_QUEUE_LEN);

    let stats_counter = StatsCounter::default();

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = bootstrapped_node.blockchain.clone();
        let stats_counter = stats_counter.clone();
        services.spawn_with_inputs("transaction", move |info, input| {
            transaction::handle_input(info, &blockchain, &tpool, &stats_counter, input)
        })
    };

    let block_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        // let clock = bootstrapped_node.clock.clone();
        let stats_counter = stats_counter.clone();
        services.spawn_future_with_inputs("block", move |info, input| {
            blockchain::handle_input(info, &blockchain, &stats_counter, &network_msgbox, input);
            futures::future::ok(())
        })
    };

    let client_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        services.spawn_with_inputs("client-query", move |info, input| {
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

        services.spawn("network", move |_info| {
            network::run(config, network_queue, channels);
        });
    }

    let leader_secret = if let Some(secret_path) = &bootstrapped_node.settings.leadership {
        Some(secure::NodeSecret::load_from_file(secret_path.as_path()))
    } else {
        None
    };

    if let Some(secret) = leader_secret {
        let tpool = tpool.clone();
        let clock = bootstrapped_node.clock.clone();
        let block_task = block_task.clone();
        let blockchain = bootstrapped_node.blockchain.clone();
        let pk = Leader {
            bft_leader: Some(BftLeader {
                sig_key: secret.block_privatekey,
            }),
            genesis_leader: None,
        };
        services.spawn("leadership", move |info| {
            leadership_task(info, pk, tpool, blockchain, clock, block_task)
        });
    }

    let rest_server = match bootstrapped_node.settings.rest {
        Some(ref rest) => {
            let context = rest::Context {
                stats_counter,
                blockchain: bootstrapped_node.blockchain.clone(),
                transaction_task: Arc::new(Mutex::new(transaction_task)),
            };
            Some(rest::start_rest_server(rest, context)?)
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
/// * gclock sync ?
///
///
fn bootstrap(initialized_node: InitializedNode) -> Result<BootstrappedNode, start_up::Error> {
    let block0 = initialized_node.block0;
    let clock = initialized_node.clock;
    let settings = initialized_node.settings;
    let storage = initialized_node.storage;

    let blockchain = start_up::load_blockchain(block0, storage)?;

    network::bootstrap(&settings.network, blockchain.clone());

    Ok(BootstrappedNode {
        settings,
        clock,
        blockchain,
    })
}

pub struct InitializedNode {
    pub settings: Settings,
    pub block0: blockcfg::Block,
    pub clock: clock::Clock,
    pub storage: start_up::NodeStorage,
}

fn initialize_node() -> Result<InitializedNode, start_up::Error> {
    use start_up::*;

    prepare_resources()?;

    let command_line_arguments = load_command_line()?;

    let node_settings = load_settings(&command_line_arguments)?;

    prepare_logger(&node_settings)?;

    let storage = prepare_storage(&node_settings)?;

    // TODO: load network module here too (if needed)

    let block0 = prepare_block_0(
        &node_settings,
        &storage,
        /* add network to fetch block0 */
    )?;

    let clock = prepare_clock(&block0)?;

    Ok(InitializedNode {
        settings: node_settings,
        block0: block0,
        clock: clock,
        storage: storage,
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
