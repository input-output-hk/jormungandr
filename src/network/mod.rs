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
mod subscription;

use crate::blockchain::BlockchainR;
use crate::intercom::{BlockMsg, ClientMsg, NetworkPropagateMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Listen, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::TaskMessageBox,
};

use self::p2p_topology::{self as p2p, P2pTopology};
use self::propagate::PropagationMap;

use futures::prelude::*;
use futures::stream;

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

/// Global state shared between all network tasks.
pub struct GlobalState {
    pub config: Configuration,
    pub topology: P2pTopology,
    pub node: p2p::Node,
    pub propagation_peers: PropagationMap,
}

type GlobalStateR = Arc<GlobalState>;

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
            propagation_peers: PropagationMap::new(),
        }
    }
}

pub struct ConnectionState {
    /// The global state shared between all connections
    pub global: GlobalStateR,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,
}

impl ConnectionState {
    fn new(global: GlobalStateR, peer: &Peer) -> Self {
        ConnectionState {
            global,
            timeout: peer.timeout,
            connection: peer.connection,
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
    let global_state = Arc::new(GlobalState::new(config));

    // open the port for listening/accepting other peers to connect too
    let listener = if let Some(public_address) = global_state
        .config
        .public_address
        .as_ref()
        .and_then(move |addr| addr.to_socketaddr())
    {
        let protocol = global_state.config.protocol;
        match protocol {
            Protocol::Grpc => {
                let listen = Listen::new(public_address, protocol);
                grpc::run_listen_socket(listen, global_state.clone(), channels.clone())
            }
            Protocol::Ntt => unimplemented!(),
        }
    } else {
        unimplemented!()
    };

    let addrs = global_state
        .config
        .trusted_addresses
        .iter()
        .filter_map(|paddr| paddr.to_socketaddr())
        .collect::<Vec<_>>();
    let state = global_state.clone();
    let connections = stream::iter_ok(addrs).for_each(move |addr| {
        let peer = Peer::new(addr, Protocol::Grpc);
        let conn_state = ConnectionState::new(state.clone(), &peer);
        let state = state.clone();
        grpc::connect(addr, conn_state, channels.clone()).map(move |(node_id, prop_handles)| {
            state.propagation_peers.insert_peer(node_id, prop_handles);
        })
    });

    let state = global_state.clone();
    let propagate = propagate_input
        .for_each(move |msg| {
            let node_ids = state.topology.view_ids();
            match msg {
                NetworkPropagateMsg::Block(header) => {
                    state.propagation_peers.propagate_block(&node_ids, header);
                }
                NetworkPropagateMsg::Message(message) => {
                    state
                        .propagation_peers
                        .propagate_message(&node_ids, message);
                }
            }
            Ok(())
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
