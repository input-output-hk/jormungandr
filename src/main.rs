#![cfg_attr(feature = "with-bench", feature(test))]
extern crate actix_net;
extern crate actix_web;
extern crate bincode;
extern crate cardano;
extern crate cardano_storage;
extern crate cbor_event;
extern crate chain_addr;
extern crate chain_core;
extern crate chain_impl_mockchain;
extern crate chain_storage;
extern crate clap;
extern crate cryptoxide;
extern crate curve25519_dalek;
extern crate exe_common;
extern crate futures;
extern crate generic_array;
extern crate sha2;
#[macro_use]
extern crate lazy_static;
extern crate native_tls;
extern crate network_core;
extern crate network_grpc;
extern crate protocol_tokio as protocol;
extern crate tower_service;

extern crate tokio;

#[cfg(test)]
extern crate quickcheck;
extern crate rand;
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

use std::sync::{mpsc::Receiver, Arc, RwLock};

use chain_impl_mockchain::{
    key::PrivateKey,
    transaction::{SignedTransaction, TransactionId},
};
use futures::sync::mpsc::UnboundedSender;
use futures::Future;

use blockcfg::{genesis_data::GenesisData, mock::Mockchain as Cardano};
//use state::State;
use blockchain::{Blockchain, BlockchainR};
use intercom::BlockMsg;
use intercom::NetworkBroadcastMsg;
use leadership::leadership_task;
use rest::v0_node_stats::StatsCounter;
use settings::Command;
use transaction::{transaction_task, TPool};
use utils::task::Tasks;

#[macro_use]
pub mod log_wrapper;

pub mod blockcfg;
pub mod blockchain;
pub mod client;
pub mod clock;
pub mod consensus;
pub mod intercom;
pub mod leadership;
pub mod network;
pub mod rest;
pub mod secure;
pub mod settings;
pub mod state;
pub mod transaction;
pub mod utils;

pub type TODO = u32;

fn node_private_key(ns: secure::NodeSecret) -> PrivateKey {
    let mut bytes = [0; 32];
    bytes.copy_from_slice(&ns.block_privatekey.as_ref()[0..32]);
    PrivateKey::normalize_bytes(bytes)
}

fn block_task(
    blockchain: BlockchainR<Cardano>,
    _clock: clock::Clock, // FIXME: use it or lose it
    r: Receiver<BlockMsg<Cardano>>,
    network_broadcast: UnboundedSender<NetworkBroadcastMsg<Cardano>>,
    stats_counter: StatsCounter,
) {
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(&blockchain, bquery, &network_broadcast, &stats_counter);
    }
}

fn startup_info(
    gd: &GenesisData,
    blockchain: &Blockchain<Cardano>,
    settings: &settings::start::Settings,
) {
    println!(
        "k={} tip={}",
        gd.epoch_stability_depth,
        blockchain.get_tip()
    );
}

// Expand the type with more variants
// when it becomes necessary to represent different error cases.
type Error = settings::Error;

fn start(settings: settings::start::Settings) -> Result<(), Error> {
    settings.log_settings.apply();

    let genesis_data = settings.read_genesis_data().unwrap();

    let clock = {
        let initial_epoch = clock::ClockEpochConfiguration {
            slot_duration: genesis_data.slot_duration,
            slots_per_epoch: genesis_data.epoch_stability_depth * 10,
        };
        clock::Clock::new(genesis_data.start_time, initial_epoch)
    };

    let leader_secret = if let Some(secret_path) = &settings.leadership {
        Some(secure::NodeSecret::load_from_file(secret_path.as_path()).unwrap())
    } else {
        None
    };
    let leader_public = if let Some(ref secret) = &leader_secret {
        Some(secret.public())
    } else {
        None
    };

    //let mut state = State::new();

    let blockchain_data = Blockchain::new(genesis_data.clone(), leader_public);

    startup_info(&genesis_data, &blockchain_data, &settings);

    let blockchain = Arc::new(RwLock::new(blockchain_data));

    let mut tasks = Tasks::new();

    // # Bootstrap phase
    //
    // done at every startup: we need to bootstrap from whatever local state (including nothing)
    // to the latest network state (or close to latest). until this happen, we don't participate in the network
    // (no block creation) and our network connection(s) is only use to download data.
    //
    // Various aspects to do, similar to hermes:
    // * download all the existing blocks
    // * verify all the downloaded blocks
    // * network / peer discoveries (?)
    // * gclock sync ?

    // Read block state
    // init storage
    // create blockchain storage

    network::bootstrap(&settings.network, blockchain.clone());

    // # Active phase
    //
    // now that we have caught up (or almost caught up) we download blocks from neighbor nodes,
    // listen to announcements and actively listen to synchronous queries
    //
    // There's two simultaenous roles to this:
    // * Leader: decided after global or local evaluation. Need to create and propagate a block
    // * Non-Leader: always. receive (pushed-) blocks from other peers, investigate the correct blockchain updates
    //
    // Also receive synchronous connection queries:
    // * new nodes subscribing to updates (blocks, transactions)
    // * client GetBlocks/Headers ...

    let tpool_data: TPool<TransactionId, SignedTransaction> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    // Validation of consensus settings should make sure that we always have
    // non-empty selection data.

    // initialize the transaction broadcast channel
    let (broadcast_sender, broadcast_receiver) = futures::sync::mpsc::unbounded();

    let stats_counter = StatsCounter::default();

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = blockchain.clone();
        let stats_counter = stats_counter.clone();
        tasks.task_create_with_inputs("transaction", move |r| {
            transaction_task(blockchain, tpool, r, stats_counter)
        })
    };

    let block_task = {
        let blockchain = blockchain.clone();
        let clock = clock.clone();
        let stats_counter = stats_counter.clone();
        tasks.task_create_with_inputs("block", move |r| {
            block_task(blockchain, clock, r, broadcast_sender, stats_counter)
        })
    };

    let client_task = {
        let blockchain = blockchain.clone();
        tasks.task_create_with_inputs("client-query", move |r| client::client_task(blockchain, r))
    };

    // ** TODO **
    // setup_network
    //  connection-events:
    //    poll:
    //      recv_transaction:
    //         check_transaction_valid
    //         add transaction to pool
    //      recv_block:
    //         check block valid
    //         try to extend blockchain with block
    //         update utxo state
    //         flush transaction pool if any txid made it
    //      get block(s):
    //         try to answer
    //
    {
        let client_msgbox = client_task.clone();
        let transaction_msgbox = transaction_task.clone();
        let block_msgbox = block_task.clone();
        let config = settings.network.clone();
        let channels = network::Channels {
            client_box: client_msgbox,
            transaction_box: transaction_msgbox,
            block_box: block_msgbox,
        };
        tasks.task_create("network", move || {
            network::run(config, broadcast_receiver, channels);
        });
    };

    if let Some(secret) = leader_secret
    // == settings::start::Leadership::Yes
    //    && leadership::selection::can_lead(&selection) == leadership::IsLeading::Yes
    {
        let tpool = tpool.clone();
        let clock = clock.clone();
        let block_task = block_task.clone();
        let blockchain = blockchain.clone();
        let pk = node_private_key(secret);
        tasks.task_create("leadership", move || {
            leadership_task(pk, tpool, blockchain, clock, block_task)
        });
    };

    let rest_server = match settings.rest {
        Some(ref rest) => Some(rest::start_rest_server(rest, stats_counter)?),
        None => None,
    };

    // periodically cleanup (custom):
    //   storage cleanup/packing
    //   tpool.gc()

    // FIXME some sort of join so that the main thread does something ...
    tasks.join();

    if let Some(server) = rest_server {
        server.stop().wait().unwrap()
    }

    Ok(())
}

fn main() {
    let command = match Command::load() {
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
        Ok(v) => v,
    };

    match command {
        Command::Start(start_settings) => {
            if let Err(error) = start(start_settings) {
                eprintln!("jormungandr error: {}", error);
                std::process::exit(1);
            }
        }
        Command::GenerateKeys => {
            use cardano::util::hex;
            let seed: Vec<u8> = std::iter::repeat_with(|| rand::random())
                .take(cardano::redeem::PRIVATEKEY_SIZE)
                .collect();
            let signing_key = cardano::redeem::PrivateKey::generate(&seed).unwrap();
            let public_key = signing_key.public();
            println!("signing_key: {}", hex::encode(signing_key.as_ref()));
            println!("public_key: {}", hex::encode(public_key.as_ref()));
        }
        Command::Init(init_settings) => {
            let genesis = GenesisData {
                start_time: init_settings.blockchain_start,
                slot_duration: init_settings.slot_duration,
                epoch_stability_depth: init_settings.epoch_stability_depth,
                initial_utxos: init_settings.initial_utxos,
                obft_leaders: init_settings.obft_leaders,
            };

            serde_yaml::to_writer(std::io::stdout(), &genesis).unwrap();
        }
    }
}
