#![cfg_attr(feature = "with-bench", feature(test))]
extern crate bincode;
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate serde_yaml;
#[macro_use(o)]
extern crate slog;
extern crate rand;
extern crate slog_async;
extern crate slog_json;
extern crate slog_term;
extern crate structopt;

extern crate cardano;
extern crate cardano_storage;
#[macro_use]
extern crate cbor_event;
extern crate exe_common;
extern crate protocol_tokio as protocol;

#[macro_use]
extern crate futures;
extern crate tokio;

extern crate cryptoxide;
extern crate curve25519_dalek;
extern crate generic_array;
extern crate sha2;

extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_connect;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_util;

extern crate actix_net;
extern crate actix_web;
extern crate native_tls;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[macro_use]
extern crate lazy_static;

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
pub mod stats;
pub mod transaction;
pub mod utils;

use settings::Settings;
//use state::State;
use blockchain::{Blockchain, BlockchainR};
use futures::sync::mpsc::UnboundedSender;
use futures::Future;
use intercom::BlockMsg;
use intercom::NetworkBroadcastMsg;
use leadership::{leadership_task, Selection};
use rest::ServerState;
use stats::SharedStats;
use transaction::{transaction_task, TPool};
use utils::task::Tasks;

use blockcfg::cardano::{Cardano, GenesisData, Transaction, TransactionId};

use std::sync::{mpsc::Receiver, Arc, RwLock};

use cardano_storage::StorageConfig;

pub type TODO = u32;

fn block_task(
    blockchain: BlockchainR<Cardano>,
    selection: Arc<Selection>,
    _clock: clock::Clock, // FIXME: use it or lose it
    r: Receiver<BlockMsg<Cardano>>,
    network_broadcast: UnboundedSender<NetworkBroadcastMsg<Cardano>>,
    shared_stats: SharedStats,
) {
    loop {
        let bquery = r.recv().unwrap();
        blockchain::process(
            &blockchain,
            &selection,
            bquery,
            &network_broadcast,
            &shared_stats,
        );
    }
}

fn startup_info(gd: &GenesisData, blockchain: &Blockchain<Cardano>, settings: &Settings) {
    println!(
        "protocol magic={} prev={} k={} tip={}",
        gd.protocol_magic,
        gd.genesis_prev,
        gd.epoch_stability_depth,
        blockchain.get_tip()
    );
    println!("consensus: {:?}", settings.consensus);
}

// Expand the type with more variants
// when it becomes necessary to represent different error cases.
type Error = settings::Error;

fn run() -> Result<(), Error> {
    // # load parameters & config
    //
    // parse the command line arguments, the config files supplied
    // and setup the initial values
    let settings = Settings::load()?;

    settings.log_settings.apply();

    let genesis_data = settings.read_genesis_data();

    let clock = {
        let initial_epoch = clock::ClockEpochConfiguration {
            slot_duration: genesis_data.slot_duration,
            slots_per_epoch: genesis_data.epoch_stability_depth * 10,
        };
        clock::Clock::new(genesis_data.start_time, initial_epoch)
    };

    let secret = secure::NodeSecret::load_from_file(settings.secret_config.as_path()).unwrap();

    //let mut state = State::new();

    let storage_config = StorageConfig::new(&settings.storage);
    let blockchain_data = Blockchain::from_storage(genesis_data.clone(), &storage_config);

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

    let tpool_data: TPool<TransactionId, Transaction> = TPool::new();
    let tpool = Arc::new(RwLock::new(tpool_data));

    // Validation of consensus settings should make sure that we always have
    // non-empty selection data.
    let selection_data =
        leadership::selection::prepare(&secret.to_public(), &settings.consensus).unwrap();
    let selection = Arc::new(selection_data);

    // initialize the transaction broadcast channel
    let (broadcast_sender, broadcast_receiver) = futures::sync::mpsc::unbounded();

    let shared_stats = SharedStats::default();

    let transaction_task = {
        let tpool = tpool.clone();
        let blockchain = blockchain.clone();
        let shared_stats = shared_stats.clone();
        tasks.task_create_with_inputs("transaction", move |r| {
            transaction_task(blockchain, tpool, r, shared_stats)
        })
    };

    let block_task = {
        let blockchain = blockchain.clone();
        let clock = clock.clone();
        let selection = Arc::clone(&selection);
        let shared_stats = shared_stats.clone();
        tasks.task_create_with_inputs("block", move |r| {
            block_task(
                blockchain,
                selection,
                clock,
                r,
                broadcast_sender,
                shared_stats,
            )
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

    if settings.leadership == settings::Leadership::Yes
        && leadership::selection::can_lead(&selection) == leadership::IsLeading::Yes
    {
        let tpool = tpool.clone();
        let clock = clock.clone();
        let selection = Arc::clone(&selection);
        let block_task = block_task.clone();
        let blockchain = blockchain.clone();
        tasks.task_create("leadership", move || {
            leadership_task(secret, selection, tpool, blockchain, clock, block_task)
        });
    };

    let rest_server = match settings.rest {
        Some(ref rest) => {
            let state = ServerState {
                stats: shared_stats,
            };
            Some(rest::start_rest_server(rest, state)?)
        }
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
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("jormungandr error: {}", e);
            std::process::exit(1);
        }
    }
}
