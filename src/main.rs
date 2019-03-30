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
extern crate sha2;
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
extern crate tokio_bus;

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
#[macro_use(o)]
extern crate slog;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
extern crate structopt;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

use std::sync::{mpsc::Receiver, Arc, Mutex, RwLock};

use chain_impl_mockchain::message::{Message, MessageId};
use futures::Future;

use blockchain::{Blockchain, BlockchainR};
use chain_core::property::Block as _;
use intercom::BlockMsg;
use leadership::leadership_task;
use rest::v0::node::stats::StatsCounter;
use settings::{start::Settings, CommandLine};
use transaction::{transaction_task, TPool};
use utils::task::{TaskBroadcastBox, Tasks};

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

// TODO: consider an appropriate size for the broadcast buffer.
// For the block task, there should hardly be a need to buffer more
// than one block as the network task should be able to broadcast the
// block notifications in time.
const BLOCK_BUS_CAPACITY: usize = 2;

fn block_task(
    blockchain: BlockchainR,
    _clock: clock::Clock, // FIXME: use it or lose it
    r: Receiver<BlockMsg>,
    stats_counter: StatsCounter,
) {
    let mut network_broadcast = TaskBroadcastBox::new(BLOCK_BUS_CAPACITY);
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(&blockchain, bquery, &mut network_broadcast, &stats_counter);
    }
}

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

fn start_services(bootstrapped_node: &BootstrappedNode) -> Result<(), start_up::Error> {
    let mut tasks = Tasks::new();

    let tpool_data: TPool<MessageId, Message> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));
    let stats_counter = StatsCounter::default();

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = bootstrapped_node.blockchain.clone();
        let stats_counter = stats_counter.clone();
        tasks.task_create_with_inputs("transaction", move |r| {
            transaction_task(blockchain, tpool, r, stats_counter)
        })
    };

    let block_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        let clock = bootstrapped_node.clock.clone();
        let stats_counter = stats_counter.clone();
        tasks.task_create_with_inputs("block", move |r| {
            block_task(blockchain, clock, r, stats_counter)
        })
    };

    let client_task = {
        let blockchain = bootstrapped_node.blockchain.clone();
        tasks.task_create_with_inputs("client-query", move |r| client::client_task(blockchain, r))
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
        tasks.task_create("network", move || {
            network::run(config, channels);
        });
    };

    let leader_secret = if let Some(secret_path) = &bootstrapped_node.settings.leadership {
        Some(secure::NodeSecret::load_from_file(secret_path.as_path()))
    } else {
        None
    };

    if let Some(secret) = leader_secret
    // == settings::start::Leadership::Yes
    //    && leadership::selection::can_lead(&selection) == leadership::IsLeading::Yes
    {
        let tpool = tpool.clone();
        let clock = bootstrapped_node.clock.clone();
        let block_task = block_task.clone();
        let blockchain = bootstrapped_node.blockchain.clone();
        let pk = chain_impl_mockchain::leadership::Leader::BftLeader(secret.block_privatekey);
        tasks.task_create("leadership", move || {
            leadership_task(pk, tpool, blockchain, clock, block_task)
        });
    };

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

    tasks.join();

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
