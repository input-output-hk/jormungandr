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
mod propagate;
mod service;

use crate::blockchain::BlockchainR;
use crate::intercom::{BlockMsg, ClientMsg, NetworkPropagateMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Listen, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::TaskMessageBox,
};

use self::p2p_topology::{self as p2p, P2pTopology};

use futures::prelude::*;
use futures::{future, stream};

use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

type Connection = SocketAddr;

struct BlockConfig;

/// all the different channels the network may need to talk to
pub struct Channels {
    pub client_box: TaskMessageBox<ClientMsg>,
    pub transaction_box: TaskMessageBox<TransactionMsg>,
    pub block_box: MessageBox<BlockMsg>,
}

impl Clone for Channels {
    fn clone(&self) -> Self {
        Channels {
            client_box: self.client_box.clone(),
            transaction_box: self.transaction_box.clone(),
            block_box: self.block_box.clone(),
        }
    }
}

/// Global state shared between all network tasks.
pub struct GlobalState {
    pub config: Configuration,
    pub topology: P2pTopology,
    pub node: p2p::Node,
}

impl GlobalState {
    /// the network global state
    pub fn new(config: Configuration) -> Self {
        let node_id = p2p_topology::NodeId::generate();
        let node_address = config
            .public_address
            .clone()
            .expect("only support the full nodes for now")
            .0
            .into();
        let mut node = p2p_topology::Node::new(node_id, node_address);

        // TODO: load the subscriptions from the config
        node.add_message_subscription(p2p_topology::InterestLevel::High);
        node.add_block_subscription(p2p_topology::InterestLevel::High);

        let topology = P2pTopology::new(node.clone());

        GlobalState {
            config,
            topology,
            node,
        }
    }
}

pub struct ConnectionState {
    /// The global state shared between all connections
    pub global: Arc<GlobalState>,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,

    /// State of the propagation subscriptions.
    pub propagation: Mutex<propagate::PeerHandles>,
}

impl ConnectionState {
    fn new_listen(global: Arc<GlobalState>, listen: &Listen) -> Self {
        ConnectionState {
            global,
            timeout: listen.timeout,
            connection: listen.connection,
            propagation: Mutex::new(propagate::PeerHandles::new()),
        }
    }

    fn new_peer(global: Arc<GlobalState>, peer: &Peer) -> Self {
        ConnectionState {
            global,
            timeout: peer.timeout,
            connection: peer.connection,
            propagation: Mutex::new(propagate::PeerHandles::new()),
        }
    }
}

pub fn run(
    config: Configuration,
    propagate_input: MessageQueue<NetworkPropagateMsg>,
    channels: Channels,
) {
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let state = Arc::new(GlobalState::new(config));

    // open the port for listening/accepting other peers to connect too
    let listener = if let Some(public_address) = state
        .config
        .public_address
        .as_ref()
        .and_then(move |addr| addr.to_socketaddr())
    {
        let protocol = state.config.protocol;
        match protocol {
            Protocol::Grpc => {
                let listen = Listen::new(public_address, protocol);
                let conn_state = ConnectionState::new_listen(state.clone(), &listen);
                grpc::run_listen_socket(public_address, conn_state, channels.clone())
            }
            Protocol::Ntt => unimplemented!(),
        }
    } else {
        unimplemented!()
    };

    let addrs = state
        .config
        .trusted_addresses
        .iter()
        .filter_map(|paddr| paddr.to_socketaddr())
        .collect::<Vec<_>>();
    let connections = stream::iter_ok(addrs).for_each(move |addr| {
        let peer = Peer::new(addr, Protocol::Grpc);
        let conn_state = ConnectionState::new_peer(state.clone(), &peer);
        let conn = grpc::run_connect_socket(addr, conn_state, channels.clone());
        conn // TODO: manage propagation peers in a map
    });

    let propagate = propagate_input
        .for_each(|msg| {
            // TODO: propagate message
            future::ok(())
        })
        .map_err(|_| {});

    tokio::run(connections.join(propagate).join(listener).map(|_| ()));
}

pub fn bootstrap(config: &Configuration, blockchain: BlockchainR) {
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
