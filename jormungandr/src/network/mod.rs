//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

pub mod bootstrap;
mod client;
mod grpc;
mod inbound;
pub mod p2p;
mod service;
mod subscription;

// Constants

mod chain_pull {
    // Size of chunks to split processing of chain pull streams.
    // Apart from sizing data chunks for intercom messages, it also
    // determines how many blocks will be requested per each GetBlocks request
    // distributed between different peers.
    //
    // This may need to be made into a configuration parameter.
    pub const CHUNK_SIZE: usize = 32;
}

use self::p2p::{
    comm::{PeerComms, Peers},
    topology::{self, P2pTopology},
};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{Blockchain as NewBlockchain, Tip};
use crate::intercom::{BlockMsg, ClientMsg, NetworkMsg, PropagateMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::{TaskMessageBox, TokioServiceInfo},
};
use futures::prelude::*;
use futures::stream;
use network_core::gossip::{Gossip, Node};
use rand::seq::SliceRandom;
use slog::Logger;
use tokio::timer::Interval;

use std::error;
use std::fmt;
use std::io;
use std::iter;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub use self::bootstrap::Error as BootstrapError;

#[derive(Debug)]
pub struct ListenError {
    cause: io::Error,
    sockaddr: SocketAddr,
}

impl fmt::Display for ListenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed to listen for connections on {}", self.sockaddr)
    }
}

impl error::Error for ListenError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(&self.cause)
    }
}

type Connection = SocketAddr;

pub enum BlockConfig {}

/// all the different channels the network may need to talk to
pub struct Channels {
    pub client_box: TaskMessageBox<ClientMsg>,
    pub transaction_box: MessageBox<TransactionMsg>,
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
    pub block0_hash: HeaderHash,
    pub config: Configuration,
    pub topology: P2pTopology,
    pub node: topology::Node,
    pub peers: Peers,
    pub logger: Logger,
}

type GlobalStateR = Arc<GlobalState>;

impl GlobalState {
    /// the network global state
    pub fn new(block0_hash: HeaderHash, config: Configuration, logger: Logger) -> Self {
        let node_address = config.public_address.clone().map(|addr| addr.0.into());
        let mut node = topology::Node::new(node_address);

        use self::p2p::topology::{NEW_BLOCKS_TOPIC, NEW_MESSAGES_TOPIC};

        for (topic, interest) in config.subscriptions.iter() {
            if topic.0 == NEW_BLOCKS_TOPIC.into() {
                node.add_block_subscription(interest.0)
            }
            if topic.0 == NEW_MESSAGES_TOPIC.into() {
                node.add_message_subscription(interest.0)
            }
        }

        let mut topology = P2pTopology::new(node.clone(), logger.clone());
        topology.set_poldercast_modules();
        topology.add_module(topology::modules::TrustedPeers::new_with(
            config
                .trusted_peers
                .iter()
                .cloned()
                .map(|trusted_peer| poldercast::Node::new_with(trusted_peer)),
        ));

        let peers = Peers::new(config.max_connections, logger.clone());

        GlobalState {
            block0_hash,
            config,
            topology,
            node,
            peers,
            logger,
        }
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

pub struct ConnectionState {
    /// The global state shared between all connections
    pub global: GlobalStateR,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,

    logger: Logger,
}

impl ConnectionState {
    fn new(global: GlobalStateR, peer: &Peer) -> Self {
        ConnectionState {
            timeout: peer.timeout,
            connection: peer.connection.clone(),
            logger: global.logger().new(o!("peer_addr" => peer.connection)),
            global,
        }
    }

    fn logger(&self) -> &Logger {
        &self.logger
    }
}

pub struct TaskParams {
    pub config: Configuration,
    pub block0_hash: HeaderHash,
    pub input: MessageQueue<NetworkMsg>,
    pub channels: Channels,
}

pub fn start(
    service_info: TokioServiceInfo,
    params: TaskParams,
) -> Result<impl Future<Item = (), Error = ()>, ListenError> {
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let input = params.input;
    let channels = params.channels;
    let logger = service_info.logger().clone();
    let global_state = Arc::new(GlobalState::new(
        params.block0_hash,
        params.config,
        logger.clone(),
    ));

    // open the port for listening/accepting other peers to connect too
    let listen = global_state.config.listen();
    use futures::future::Either;
    let listener = if let Some(listen) = listen {
        match listen.protocol {
            Protocol::Grpc => Either::A(grpc::run_listen_socket(
                listen,
                global_state.clone(),
                channels.clone(),
            )?),
            Protocol::Ntt => unimplemented!(),
        }
    } else {
        Either::B(futures::future::ok(()))
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
        info!(conn_state.logger(), "connecting to initial gossip peer");
        service_info.spawn(
            client::connect(conn_state, conn_channels.clone())
                .and_then(move |(client, mut comms)| {
                    // TODO
                    let node_id = client.remote_node_id();
                    let gossip = Gossip::from_nodes(iter::once(state.node.clone()));
                    match comms.try_send_gossip(gossip) {
                        Ok(()) => state.peers.insert_peer(node_id, comms),
                        Err(e) => {
                            warn!(
                                client.logger(),
                                "gossiping to peer failed just after connection: {:?}", e
                            );
                            return Err(());
                        }
                    }
                    Ok(client)
                })
                .and_then(|client| client),
        );
        Ok(())
    });

    let handle_cmds = handle_network_input(input, global_state.clone(), channels.clone());

    let gossip_err_logger = logger.clone();
    // TODO: get gossip propagation interval from configuration
    let gossip = Interval::new_interval(Duration::from_secs(10))
        .map_err(move |e| {
            error!(gossip_err_logger, "interval timer error: {:?}", e);
        })
        .for_each(move |_| {
            send_gossip(global_state.clone(), channels.clone());
            Ok(())
        });

    Ok(listener.join4(connections, handle_cmds, gossip).map(|_| ()))
}

fn handle_network_input(
    input: MessageQueue<NetworkMsg>,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    input.for_each(move |msg| match msg {
        NetworkMsg::Propagate(msg) => {
            handle_propagation_msg(msg, state.clone(), channels.clone());
            Ok(())
        }
        NetworkMsg::GetBlocks(block_ids) => {
            state.peers.fetch_blocks(block_ids);
            Ok(())
        }
        NetworkMsg::GetNextBlock(node_id, block_id) => {
            state.peers.solicit_blocks(node_id, vec![block_id]);
            Ok(())
        }
        NetworkMsg::PullHeaders { node_id, from, to } => {
            state.peers.pull_headers(node_id, from.into(), to);
            Ok(())
        }
    })
}

fn handle_propagation_msg(msg: PropagateMsg, state: GlobalStateR, channels: Channels) {
    trace!(state.logger(), "to propagate: {:?}", &msg);
    let nodes = state.topology.view().collect::<Vec<_>>();
    debug!(
        state.logger(),
        "will propagate to: {:?}",
        nodes.iter().map(|node| node.id()).collect::<Vec<_>>()
    );
    let res = match msg {
        PropagateMsg::Block(ref header) => state.peers.propagate_block(nodes, header.clone()),
        PropagateMsg::Fragment(ref fragment) => {
            state.peers.propagate_fragment(nodes, fragment.clone())
        }
    };
    // If any nodes selected for propagation are not in the
    // active subscriptions map, connect to them and deliver
    // the item.
    if let Err(unreached_nodes) = res {
        for node in unreached_nodes {
            let msg = msg.clone();
            connect_and_propagate_with(
                node,
                state.clone(),
                channels.clone(),
                |handles| match msg {
                    PropagateMsg::Block(header) => handles
                        .try_send_block_announcement(header)
                        .map_err(|e| e.kind()),
                    PropagateMsg::Fragment(fragment) => {
                        handles.try_send_fragment(fragment).map_err(|e| e.kind())
                    }
                },
            );
        }
    }
}

fn send_gossip(state: GlobalStateR, channels: Channels) {
    for node in state.topology.view() {
        let gossip = Gossip::from_nodes(state.topology.select_gossips(&node));
        debug!(state.logger(), "sending gossip to node {}", node.id());
        let res = state.peers.propagate_gossip_to(node.id(), gossip);
        if let Err(gossip) = res {
            connect_and_propagate_with(node, state.clone(), channels.clone(), |handles| {
                handles.try_send_gossip(gossip).map_err(|e| e.kind())
            });
        }
    }
}

fn connect_and_propagate_with<F>(
    node: topology::Node,
    state: GlobalStateR,
    channels: Channels,
    once_connected: F,
) where
    F: FnOnce(&mut PeerComms) -> Result<(), p2p::comm::ErrorKind> + Send + 'static,
{
    let addr = match node.address() {
        Some(addr) => addr,
        None => {
            debug!(
                state.logger(),
                "ignoring P2P node without an IP address: {:?}", node
            );
            return;
        }
    };
    let node_id = node.id();
    let peer = Peer::new(addr, Protocol::Grpc);
    let conn_state = ConnectionState::new(state.clone(), &peer);
    let logger = conn_state
        .logger()
        .new(o!("node_id" => node_id.to_string()));
    debug!(logger, "connecting to node");
    let err_state = state.clone();
    let cf = client::connect(conn_state, channels.clone())
        .and_then(move |(client, mut comms)| {
            let connected_node_id = client.remote_node_id();
            if connected_node_id == node_id {
                let res = once_connected(&mut comms);
                match res {
                    Ok(()) => (),
                    Err(e) => {
                        info!(
                            client.logger(),
                            "propagation to peer failed just after connection: {:?}", e
                        );
                        return Err(());
                    }
                }
            } else {
                info!(
                    client.logger(),
                    "peer responded with different node id: {}", connected_node_id
                );
                state.peers.remove_peer(node_id);
            };

            state.peers.insert_peer(connected_node_id, comms);

            Ok(client)
        })
        .and_then(|client| client)
        .then(move |res| {
            info!(logger, "client P2P connection closed");
            if let Err(()) = res {
                err_state.topology.evict_node(node_id);
                debug!(logger, "evicted node");
            }
            Ok(())
        });
    tokio::spawn(cf);
}

fn trusted_peers_shuffled(config: &Configuration) -> Vec<SocketAddr> {
    let mut peers = config
        .trusted_peers
        .iter()
        .filter_map(|peer| peer.to_socketaddr())
        .collect::<Vec<_>>();
    let mut rng = rand::thread_rng();
    peers.shuffle(&mut rng);
    peers
}

pub fn bootstrap(
    config: &Configuration,
    blockchain: NewBlockchain,
    branch: Tip,
    logger: &Logger,
) -> Result<bool, bootstrap::Error> {
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }

    if config.trusted_peers.is_empty() {
        warn!(logger, "No trusted peers joinable to bootstrap the network");
    }

    let mut bootstrapped = false;

    for address in trusted_peers_shuffled(&config) {
        let logger = logger.new(o!("peer_addr" => address.to_string()));
        let peer = Peer::new(address, Protocol::Grpc);
        let res = bootstrap::bootstrap_from_peer(peer, blockchain.clone(), branch.clone(), &logger);

        match res {
            Err(bootstrap::Error::Connect(err)) => {
                warn!(logger, "Unable to reach peer for initial bootstrap" ; "reason" => err.to_string());
            }
            Err(err) => {
                warn!(logger, "with initial bootstrap" ; "reason" => err.to_string());
            }
            Ok(_) => {
                info!(logger, "initial bootstrap completed");
                bootstrapped = true;
                break;
            }
        }
    }

    Ok(bootstrapped)
}

/// Queries the trusted peers for a block identified with the hash.
/// The calling thread is blocked until the block is retrieved.
/// This function is called during blockchain initialization
/// to retrieve the genesis block.
pub fn fetch_block(
    config: &Configuration,
    hash: &HeaderHash,
    logger: &Logger,
) -> Result<Block, FetchBlockError> {
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }

    if config.trusted_peers.is_empty() {
        return Err(FetchBlockError::NoTrustedPeers);
    }

    let mut block = None;

    let logger = logger.new(o!("block" => hash.to_string()));

    for address in trusted_peers_shuffled(&config) {
        let logger = logger.new(o!("peer_address" => address.to_string()));
        let peer = Peer::new(address, Protocol::Grpc);
        match grpc::fetch_block(peer, hash, &logger) {
            Err(err) => {
                warn!(logger, "error downloading block" ; "reason" => err.to_string());
            }
            Ok(b) => {
                info!(logger, "initial bootstrap completed");
                block = Some(b);
                break;
            }
        }
    }

    if let Some(block) = block {
        Ok(block)
    } else {
        Err(FetchBlockError::CouldNotDownloadBlock {
            block: hash.to_owned(),
        })
    }
}

custom_error! {
    pub FetchBlockError
        NoTrustedPeers = "no trusted peers specified",
        CouldNotDownloadBlock { block: HeaderHash } = "could not download block hash {block}",
}
