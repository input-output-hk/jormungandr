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

use std::{net::SocketAddr, sync::Arc, time::Duration};

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

pub struct GlobalState {
    pub config: Arc<Configuration>,
    pub channels: Channels,
    pub topology: P2pTopology,
    pub node: p2p::Node,
}

impl GlobalState {
    /// the network global state
    pub fn new(config: &Configuration, channels: Channels) -> Self {
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

        let p2p_topology = P2pTopology::new(node.clone());

        let arc_config = Arc::new(config.clone());
        GlobalState {
            config: arc_config,
            channels: channels,
            topology: p2p_topology,
            node,
        }
    }
}

impl Clone for GlobalState {
    fn clone(&self) -> Self {
        GlobalState {
            config: self.config.clone(),
            channels: self.channels.clone(),
            topology: self.topology.clone(),
            node: self.node.clone(),
        }
    }
}

#[derive(Clone)]
pub struct ConnectionState {
    /// The global network configuration
    pub global_network_configuration: Arc<Configuration>,

    /// the channels the connection will need to have to
    /// send messages too
    pub channels: Channels,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,

    pub connected: Option<Connection>,

    /// Network topology reference.
    pub topology: P2pTopology,

    /// Node inside network topology.
    pub node: p2p::Node,

    /// State of the propagation subscriptions, if established.
    pub propagation: propagate::PeerHandlesR,
}

impl ConnectionState {
    fn new_listen(global: &GlobalState, listen: &Listen) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: listen.timeout,
            connection: listen.connection,
            connected: None,
            topology: global.topology.clone(),
            node: global.node.clone(),
            propagation: propagate::PeerHandles::new(),
        }
    }

    fn new_peer(global: &GlobalState, peer: &Peer) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: peer.timeout,
            connection: peer.connection,
            connected: None,
            topology: global.topology.clone(),
            node: global.node.clone(),
            propagation: propagate::PeerHandles::new(),
        }
    }
    fn connected(mut self, connection: Connection) -> Self {
        self.connected = Some(connection);
        self
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

    let state_connection = state.clone();
    let addrs = config
        .trusted_addresses
        .iter()
        .filter_map(|paddr| paddr.to_socketaddr())
        .collect::<Vec<_>>();
    let connections = stream::iter_ok(addrs).for_each(move |addr| {
        let peer = Peer::new(addr, Protocol::Grpc);
        let (conn, _propagation) = grpc::run_connect_socket(peer, state_connection.clone());
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
