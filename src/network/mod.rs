//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

mod grpc;
// TODO: to be ported
//mod ntt;
pub mod p2p_topology;
mod service;

use std::{sync::Arc, time::Duration};

use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;
use crate::intercom::{BlockMsg, ClientMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Listen, Peer, Protocol};
use crate::utils::task::TaskMessageBox;

use self::p2p_topology::P2pTopology;
use chain_core::property;
use futures::future;
use futures::prelude::*;
use futures::stream::{self, Stream};
use std::net::SocketAddr;

type Connection = SocketAddr;

/// all the different channels the network may need to talk to
pub struct Channels<B: BlockConfig> {
    pub client_box: TaskMessageBox<ClientMsg<B>>,
    pub transaction_box: TaskMessageBox<TransactionMsg<B>>,
    pub block_box: TaskMessageBox<BlockMsg<B>>,
}

impl<B: BlockConfig> Clone for Channels<B> {
    fn clone(&self) -> Self {
        Channels {
            client_box: self.client_box.clone(),
            transaction_box: self.transaction_box.clone(),
            block_box: self.block_box.clone(),
        }
    }
}

pub struct GlobalState<B: BlockConfig> {
    pub config: Arc<Configuration>,
    pub channels: Channels<B>,
    pub topology: P2pTopology,
}

impl<B: BlockConfig> GlobalState<B> {
    /// the network global state
    pub fn new(config: &Configuration, channels: Channels<B>) -> Self {
        let node_id = p2p_topology::Id::generate(&mut rand::thread_rng());
        let node_address = config
            .public_address
            .clone()
            .expect("only support the full nodes for now")
            .0
            .into();
        let mut node = p2p_topology::Node::new(node_id, node_address);

        // TODO: load the subscriptions from the config
        p2p_topology::add_transaction_subscription(&mut node, p2p_topology::InterestLevel::High);
        p2p_topology::add_block_subscription(&mut node, p2p_topology::InterestLevel::High);

        let p2p_topology = P2pTopology::new(node);

        let arc_config = Arc::new(config.clone());
        GlobalState {
            config: arc_config,
            channels: channels,
            topology: p2p_topology,
        }
    }
}

impl<B: BlockConfig> Clone for GlobalState<B> {
    fn clone(&self) -> Self {
        GlobalState {
            config: self.config.clone(),
            channels: self.channels.clone(),
            topology: self.topology.clone(),
        }
    }
}

pub struct ConnectionState<B: BlockConfig> {
    /// The global network configuration
    pub global_network_configuration: Arc<Configuration>,

    /// the channels the connection will need to have to
    /// send messages too
    pub channels: Channels<B>,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,

    pub connected: Option<Connection>,
}

impl<B: BlockConfig> Clone for ConnectionState<B> {
    fn clone(&self) -> Self {
        ConnectionState {
            global_network_configuration: self.global_network_configuration.clone(),
            channels: self.channels.clone(),
            timeout: self.timeout,
            connection: self.connection.clone(),
            connected: self.connected.clone(),
        }
    }
}

impl<B: BlockConfig> ConnectionState<B> {
    fn new_listen(global: &GlobalState<B>, listen: Listen) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: listen.timeout,
            connection: listen.connection,
            connected: None,
        }
    }

    fn new_peer(global: &GlobalState<B>, peer: Peer) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: peer.timeout,
            connection: peer.connection,
            connected: None,
        }
    }
    fn connected(mut self, connection: Connection) -> Self {
        self.connected = Some(connection);
        self
    }
}

pub fn run<B>(config: Configuration, channels: Channels<B>)
where
    B: BlockConfig + 'static,
{
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let state = GlobalState::new(&config, channels);
    let protocol = config.protocol;

    let state_listener = state.clone();
    // open the port for listening/accepting other peers to connect too
    let listener = if let Some(public_address) = config
        .public_address
        .and_then(move |addr| addr.to_socketaddr())
    {
        let protocol = protocol.clone();
        match protocol.clone() {
            Protocol::Grpc => {
                let listen = Listen::new(public_address, protocol);
                grpc::run_listen_socket(public_address, listen, state_listener)
            }
            Protocol::Ntt => unimplemented!(),
        }
    } else {
        unimplemented!()
    };

    let connections = stream::iter_ok(config.trusted_addresses).for_each(move |_| {
        let protocol = protocol.clone();
        match protocol {
            Protocol::Ntt => {
                unimplemented!();
                // ntt::run_connect_socket(sockaddr, peer, state_connection.clone()),
                #[allow(unreachable_code)]
                future::ok(())
            }
            Protocol::Grpc => unimplemented!(),
        }
    });

    tokio::run(connections.join(listener).map(|_| ()));
}

pub fn bootstrap<B>(config: &Configuration, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
{
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }
    let peer = config.trusted_addresses.iter().next();
    match peer.and_then(|a| a.to_socketaddr()) {
        Some(address) => {
            let peer = Peer::new(address, Protocol::Grpc);
            grpc::bootstrap_from_peer(peer, blockchain)
        }
        None => {
            warn!("no gRPC peers specified, skipping bootstrap");
        }
    }
}
