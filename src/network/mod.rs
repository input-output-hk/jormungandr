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

use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::BlockchainR;
use crate::intercom::{BlockMsg, ClientMsg, NetworkMsg, PropagateMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Listen, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::TaskMessageBox,
};

use self::p2p_topology::{self as p2p, P2pTopology};
use self::propagate::{PeerHandles, PropagationMap};

use network_core::{
    error as core_error,
    gossip::{Gossip, Node},
};

use futures::prelude::*;
use futures::{future, stream};
use tokio::timer::Interval;

use std::{error::Error, iter, net::SocketAddr, sync::Arc, time::Duration};

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
        let node_id = config.public_id.unwrap_or(p2p_topology::NodeId::generate());
        info!("our node id: {}", node_id);
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

        let mut topology = P2pTopology::new(node.clone());
        topology.set_poldercast_modules();
        topology.add_module(p2p::modules::TrustedPeers::new_with(
            config.trusted_peers.iter().cloned().map(|trusted_peer| {
                poldercast::Node::new(trusted_peer.id.0, trusted_peer.address.0)
            }),
        ));

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

pub fn run(config: Configuration, input: MessageQueue<NetworkMsg>, channels: Channels) {
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
        .topology
        .view()
        .filter_map(|paddr| paddr.address())
        .collect::<Vec<_>>();
    let state = global_state.clone();
    let conn_channels = channels.clone();
    let connections = stream::iter_ok(addrs).for_each(move |addr| {
        let peer = Peer::new(addr, Protocol::Grpc);
        let conn_state = ConnectionState::new(state.clone(), &peer);
        let state = state.clone();
        grpc::connect(conn_state, conn_channels.clone()).map(move |(node_id, mut prop_handles)| {
            debug!("connected to {} at {}", node_id, addr);
            let gossip = Gossip::from_nodes(iter::once(state.node.clone()));
            match prop_handles.try_send_gossip(gossip) {
                Ok(()) => state.propagation_peers.insert_peer(node_id, prop_handles),
                Err(e) => {
                    info!(
                        "gossiping to peer {} failed just after connection: {:?}",
                        node_id, e
                    );
                }
            }
        })
    });

    let handle_cmds = handle_network_input(input, global_state.clone(), channels.clone());

    // TODO: get gossip propagation interval from configuration
    let gossip = Interval::new_interval(Duration::from_secs(10))
        .map_err(|e| {
            error!("interval timer error: {:?}", e);
        })
        .for_each(move |_| {
            send_gossip(global_state.clone(), channels.clone());
            Ok(())
        });

    tokio::run(listener.join4(connections, handle_cmds, gossip).map(|_| ()));
}

fn handle_network_input(
    input: MessageQueue<NetworkMsg>,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    input.for_each(move |msg| match msg {
        NetworkMsg::Propagate(msg) => {
            future::Either::A(handle_propagation_msg(msg, state.clone(), channels.clone()))
        }
        NetworkMsg::GetBlocks(node_id, headers) => future::Either::B({
            unimplemented!();
            future::ok(())
        }),
    })
}

fn handle_propagation_msg(
    msg: PropagateMsg,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    debug!("to propagate: {:?}", &msg);
    let nodes = state.topology.view().collect::<Vec<_>>();
    debug!(
        "will propagate to: {:?}",
        nodes.iter().map(|node| node.id()).collect::<Vec<_>>()
    );
    let res = match msg {
        PropagateMsg::Block(ref header) => state
            .propagation_peers
            .propagate_block(nodes, header.clone()),
        PropagateMsg::Message(ref message) => state
            .propagation_peers
            .propagate_message(nodes, message.clone()),
    };
    // If any nodes selected for propagation are not in the
    // active subscriptions map, connect to them and deliver
    // the item.
    future::result(res.map_err(|unreached_nodes| {
        for node in unreached_nodes {
            let msg = msg.clone();
            connect_and_propagate_with(
                node,
                state.clone(),
                channels.clone(),
                |handles| match msg {
                    PropagateMsg::Block(header) => {
                        handles.try_send_block(header).map_err(|e| e.kind())
                    }
                    PropagateMsg::Message(message) => {
                        handles.try_send_message(message).map_err(|e| e.kind())
                    }
                },
            );
        }
    }))
}

fn send_gossip(state: GlobalStateR, channels: Channels) {
    for node in state.topology.view() {
        let gossip = Gossip::from_nodes(state.topology.select_gossips(&node));
        debug!("sending gossip to node {}", node.id());
        let res = state
            .propagation_peers
            .propagate_gossip_to(node.id(), gossip);
        if let Err(gossip) = res {
            connect_and_propagate_with(node, state.clone(), channels.clone(), |handles| {
                handles.try_send_gossip(gossip).map_err(|e| e.kind())
            });
        }
    }
}

fn connect_and_propagate_with<F>(
    node: p2p::Node,
    state: GlobalStateR,
    channels: Channels,
    once_connected: F,
) where
    F: FnOnce(&mut PeerHandles) -> Result<(), propagate::ErrorKind> + Send + 'static,
{
    let addr = match node.address() {
        Some(addr) => addr,
        None => {
            info!("ignoring P2P node without an IP address: {:?}", node);
            return;
        }
    };
    let node_id = node.id();
    debug!("connecting to node {} at {}", node_id, addr);
    let peer = Peer::new(addr, Protocol::Grpc);
    let conn_state = ConnectionState::new(state.clone(), &peer);
    let state = state.clone();
    let cf =
        grpc::connect(conn_state, channels.clone()).map(move |(connected_node_id, mut handles)| {
            if connected_node_id == node_id {
                let res = once_connected(&mut handles);
                match res {
                    Ok(()) => (),
                    Err(e) => {
                        info!(
                            "propagation to peer {} failed just after connection: {:?}",
                            connected_node_id, e
                        );
                        return;
                    }
                }
            } else {
                info!(
                    "peer at {} responded with different node id: {}",
                    addr, connected_node_id
                );
            };

            state
                .propagation_peers
                .insert_peer(connected_node_id, handles);
        });
    tokio::spawn(cf);
}

fn first_trusted_peer_address(config: &Configuration) -> Option<SocketAddr> {
    config
        .trusted_peers
        .iter()
        .filter_map(|peer| peer.address.to_socketaddr())
        .next()
}

pub fn bootstrap(config: &Configuration, blockchain: BlockchainR) {
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }
    match first_trusted_peer_address(config) {
        Some(address) => {
            let peer = Peer::new(address, Protocol::Grpc);
            grpc::bootstrap_from_peer(peer, blockchain)
        }
        None => {
            warn!("no gRPC peers specified, skipping bootstrap");
        }
    }
}

/// Queries the trusted peers for a block identified with the hash.
/// The calling thread is blocked until the block is retrieved.
/// This function is called during blockchain initialization
/// to retrieve the genesis block.
pub fn fetch_block(config: &Configuration, hash: &HeaderHash) -> Result<Block, FetchBlockError> {
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }
    match first_trusted_peer_address(config) {
        None => Err(FetchBlockError::NoTrustedPeers),
        Some(address) => {
            let peer = Peer::new(address, Protocol::Grpc);
            grpc::fetch_block(peer, hash)
        }
    }
}

custom_error! {
    pub FetchBlockError
        NoTrustedPeers = "no trusted peers specified",
        Connect { source: Box<Error> } = "connection to peer failed",
        GetBlocks { source: core_error::Error } = "block request failed",
        NoBlocks = "no blocks in the stream",
}
