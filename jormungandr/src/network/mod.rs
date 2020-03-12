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

use thiserror::Error;

// Constants

mod buffer_sizes {
    pub mod inbound {
        // Size of buffer for processing of header push/pull streams.
        pub const HEADERS: usize = 32;

        // The maximum number of blocks to buffer from an incoming stream
        // (GetBlocks response or an UploadBlocks request)
        // while waiting for the block task to become ready to process
        // the next block.
        pub const BLOCKS: usize = 8;

        // The maximum number of fragments to buffer from an incoming subscription
        // while waiting for the fragment task to become ready to process them.
        pub const FRAGMENTS: usize = 128;
    }
    pub mod outbound {
        // Size of buffer for outbound header streams.
        pub const HEADERS: usize = 32;

        // The maximum number of blocks to buffer for an outbound stream
        // (GetBlocks response or an UploadBlocks request)
        // before the client request task producing them gets preempted.
        pub const BLOCKS: usize = 8;
    }
}

mod concurrency_limits {
    // How many concurrent requests are permitted per client connection
    pub const CLIENT_REQUESTS: usize = 256;
}

use self::client::ConnectError;
use self::p2p::{comm::Peers, P2pTopology};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{Blockchain as NewBlockchain, Tip};
use crate::intercom::{BlockMsg, ClientMsg, NetworkMsg, PropagateMsg, TransactionMsg};
use crate::log;
use crate::settings::start::network::{Configuration, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::TokioServiceInfo,
};
use chain_network::data::gossip::Gossip;
use futures::future;
use futures::future::Either::{A, B};
use futures::prelude::*;
use futures::stream;
use poldercast::StrikeReason;
use rand::seq::SliceRandom;
use slog::Logger;
use tokio02::time::Interval;

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::error;
use std::fmt;
use std::io;
use std::iter;
use std::net::SocketAddr;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;
use std::time::Duration;

pub use self::bootstrap::Error as BootstrapError;
use crate::stats_counter::StatsCounter;

#[derive(Debug)]
pub struct ListenError {
    cause: io::Error,
    sockaddr: SocketAddr,
}

impl fmt::Display for ListenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "failed to listen for connections on {}: {}",
            self.sockaddr, self.cause
        )
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
    pub client_box: MessageBox<ClientMsg>,
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
    pub stats_counter: StatsCounter,
    pub topology: P2pTopology,
    pub peers: Peers,
    pub executor: TaskExecutor,
    pub logger: Logger,
}

type GlobalStateR = Arc<GlobalState>;

impl GlobalState {
    /// the network global state
    pub fn new(
        block0_hash: HeaderHash,
        config: Configuration,
        topology: P2pTopology,
        stats_counter: StatsCounter,
        executor: TaskExecutor,
        logger: Logger,
    ) -> Self {
        let peers = Peers::new(config.max_connections, logger.clone());

        GlobalState {
            block0_hash,
            config,
            stats_counter,
            topology,
            peers,
            executor,
            logger,
        }
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Item = (), Error = ()> + Send + 'static,
    {
        self.executor.spawn(f)
    }

    fn inc_client_count(&self) {
        self.stats_counter.add_peer_connected_cnt(1);
    }

    fn dec_client_count(&self) {
        let prev_count = self.stats_counter.sub_peer_connected_cnt(1);
        assert!(prev_count != 0);
    }

    fn client_count(&self) -> usize {
        self.stats_counter.peer_connected_cnt()
    }

    // How many client connections to bump when a new one is about to be
    // established
    fn num_clients_to_bump(&self) -> usize {
        let count = self.stats_counter.peer_connected_cnt();
        if count > self.config.max_client_connections {
            count - self.config.max_client_connections
        } else {
            0
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

    fn peer(&self) -> Peer {
        Peer::with_timeout(self.connection, self.timeout)
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
    topology: P2pTopology,
    stats_counter: StatsCounter,
) -> impl Future<Item = (), Error = ()> {
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let input = params.input;
    let channels = params.channels;
    let global_state = Arc::new(GlobalState::new(
        params.block0_hash,
        params.config,
        topology,
        stats_counter,
        service_info.executor().clone(),
        service_info.logger().clone(),
    ));

    // open the port for listening/accepting other peers to connect too
    let listen = global_state.config.listen();
    use futures::future::Either;
    let listener = if let Some(listen) = listen {
        match listen.protocol {
            Protocol::Grpc => {
                match grpc::run_listen_socket(&listen, global_state.clone(), channels.clone()) {
                    Ok(future) => Either::A(future),
                    Err(e) => {
                        error!(
                            service_info.logger(),
                            "failed to listen for P2P connections at {}", listen.connection;
                            "reason" => %e);
                        Either::B(future::err(()))
                    }
                }
            }
            Protocol::Ntt => unimplemented!(),
        }
    } else {
        Either::B(future::ok(()))
    };

    global_state.spawn(start_gossiping(global_state.clone(), channels.clone()));

    let handle_cmds = handle_network_input(input, global_state.clone(), channels.clone());

    let gossip_err_logger = global_state.logger.clone();
    let tp2p = global_state.topology.clone();

    if let Some(interval) = global_state.config.topology_force_reset_interval.clone() {
        service_info.run_periodic("force reset topology", interval, move || {
            tp2p.force_reset_layers::<Infallible>()
        });
    }

    let gossip = Interval::new_interval(global_state.config.gossip_interval.clone())
        .map_err(move |e| {
            error!(gossip_err_logger, "interval timer error: {:?}", e);
        })
        .for_each(move |_| send_gossip(global_state.clone(), channels.clone()));

    listener.join3(handle_cmds, gossip).map(|_| ())
}

fn handle_network_input(
    input: MessageQueue<NetworkMsg>,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    input.for_each(move |msg| match msg {
        NetworkMsg::Propagate(msg) => A(A(handle_propagation_msg(
            msg,
            state.clone(),
            channels.clone(),
        ))),
        NetworkMsg::GetBlocks(block_ids) => A(B(state.peers.fetch_blocks(block_ids))),
        NetworkMsg::GetNextBlock(node_id, block_id) => {
            B(A(state.peers.solicit_blocks(node_id, vec![block_id])))
        }
        NetworkMsg::PullHeaders { node_id, from, to } => {
            B(B(A(state.peers.pull_headers(node_id, from.into(), to))))
        }
        NetworkMsg::PeerInfo(reply) => {
            B(B(B(state.peers.infos().map(|infos| reply.reply_ok(infos)))))
        }
    })
}

fn handle_propagation_msg(
    msg: PropagateMsg,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    let prop_state = state.clone();
    let send_to_peers = match msg {
        PropagateMsg::Block(ref header) => {
            debug!(state.logger(), "block to propagate"; "hash" => %header.hash());
            let header = header.clone();
            let future = state
                .topology
                .view(poldercast::Selection::Topic {
                    topic: p2p::topic::BLOCKS,
                })
                .and_then(move |view| prop_state.peers.propagate_block(view.peers, header));
            A(future)
        }
        PropagateMsg::Fragment(ref fragment) => {
            debug!(state.logger(), "fragment to propagate"; "hash" => %fragment.hash());
            let fragment = fragment.clone();
            let future = state
                .topology
                .view(poldercast::Selection::Topic {
                    topic: p2p::topic::MESSAGES,
                })
                .and_then(move |view| prop_state.peers.propagate_fragment(view.peers, fragment));
            B(future)
        }
    };
    // If any nodes selected for propagation are not in the
    // active subscriptions map, connect to them and deliver
    // the item.
    send_to_peers.then(move |res| {
        if let Err(unreached_nodes) = res {
            debug!(
                state.logger(),
                "will try to connect to {} of the peers not immediately reachable for propagation",
                unreached_nodes.len(),
            );
            for node in unreached_nodes {
                let mut options = p2p::comm::ConnectOptions::default();
                match &msg {
                    PropagateMsg::Block(header) => {
                        options.pending_block_announcement = Some(header.clone());
                    }
                    PropagateMsg::Fragment(fragment) => {
                        options.pending_fragment = Some(fragment.clone());
                    }
                };
                connect_and_propagate(node, state.clone(), channels.clone(), options);
            }
        }
        Ok(())
    })
}

fn start_gossiping(state: GlobalStateR, channels: Channels) -> impl Future<Item = (), Error = ()> {
    let config = &state.config;
    let topology = state.topology.clone();
    let conn_state = state.clone();
    // inject the trusted peers as initial gossips, this will make the node
    // gossip with them at least at the beginning
    topology
        .accept_gossips(
            (*config.profile.id()).into(),
            config
                .trusted_peers
                .iter()
                .map(|tp| {
                    let mut builder = poldercast::NodeProfileBuilder::new();
                    builder.id(tp.id.clone().into());
                    builder.address(tp.address.clone().into());
                    builder.build()
                })
                .map(p2p::Gossip::from)
                .collect::<Vec<p2p::Gossip>>()
                .into(),
        )
        .and_then(move |()| topology.view(poldercast::Selection::Any))
        .and_then(move |view| {
            for node in view.peers {
                let self_node = view.self_node.clone();
                let gossip = Gossip::from_nodes(iter::once(self_node.into()));
                let options = p2p::comm::ConnectOptions {
                    pending_gossip: Some(gossip),
                    ..Default::default()
                };
                connect_and_propagate(node, conn_state.clone(), channels.clone(), options);
            }
            Ok(())
        })
}

fn send_gossip(state: GlobalStateR, channels: Channels) -> impl Future<Item = (), Error = ()> {
    let topology = state.topology.clone();
    let logger = state.logger().new(o!(log::KEY_SUB_TASK => "send_gossip"));
    topology
        .view(poldercast::Selection::Any)
        .and_then(move |view| {
            let peers = view.peers;
            debug!(logger, "sending gossip to {} peers", peers.len());
            stream::iter_ok(peers).for_each(move |node| {
                let peer_id = node.id();
                let state_prop = state.clone();
                let state_err = state.clone();
                let channels_err = channels.clone();
                topology
                    .initiate_gossips(peer_id)
                    .and_then(move |gossips| {
                        state_prop
                            .peers
                            .propagate_gossip_to(peer_id, Gossip::from(gossips))
                    })
                    .then(move |res| {
                        if let Err(gossip) = res {
                            let options = p2p::comm::ConnectOptions {
                                pending_gossip: Some(gossip),
                                ..Default::default()
                            };
                            connect_and_propagate(node, state_err, channels_err, options);
                        }
                        Ok(())
                    })
            })
        })
}

fn connect_and_propagate(
    node: p2p::Node,
    state: GlobalStateR,
    channels: Channels,
    mut options: p2p::comm::ConnectOptions,
) {
    let addr = match node.address() {
        Some(addr) => addr,
        None => {
            debug!(
                state.logger(),
                "ignoring P2P node without an IP address" ;
                "node" => %node.id()
            );
            return;
        }
    };
    options.evict_clients = state.num_clients_to_bump();
    let node_id = node.id();
    assert_ne!(
        node_id,
        state.topology.node_id(),
        "topology tells the node to connect to itself"
    );
    let peer = Peer::new(addr);
    let conn_state = ConnectionState::new(state.clone(), &peer);
    let conn_logger = conn_state
        .logger()
        .new(o!("node_id" => node_id.to_string()));
    info!(conn_logger, "connecting to peer");
    let (handle, connecting) = client::connect(conn_state, channels.clone());
    let spawn_state = state.clone();
    let conn_err_state = state.clone();
    let cf = state.peers.add_connecting(node_id, handle, options)
        .and_then(|()| connecting)
        .or_else(move |e| {
            use crate::network::grpc::ConnectError::*;

            let benign = match e {
                ConnectError::Connect(Failed(e)) => {
                    if let Some(e) = e.connect_error() {
                        info!(conn_logger, "failed to connect to peer"; "reason" => %e);
                    } else if let Some(e) = e.http_error() {
                        info!(conn_logger, "failed to establish an HTTP connection with the peer"; "reason" => %e);
                    } else {
                        info!(conn_logger, "gRPC connection to peer failed"; "reason" => %e);
                    }
                    false
                }
                ConnectError::Connect(TimedOut) => {
                    info!(conn_logger, "connection to peer timed out");
                    false
                }
                ConnectError::Canceled => {
                    debug!(conn_logger, "connection to peer has been canceled");
                    true
                }
                _ => {
                    info!(conn_logger, "connection to peer failed"; "reason" => %e);
                    false
                }
            };
            if !benign {
                let future = conn_err_state
                    .topology
                    .report_node(node_id, StrikeReason::CannotConnect)
                    .join(conn_err_state.peers.remove_peer(node_id))
                    .and_then(|_| future::err(()));
                A(future)
            } else {
                B(future::err(()))
            }
        })
        .and_then(move |client| {
            let connected_node_id = client.remote_node_id();
            if connected_node_id != node_id {
                info!(
                    client.logger(),
                    "peer node ID differs from the expected {}", node_id
                );
                let report_and_fail = state
                    .topology
                    .report_node(node_id, StrikeReason::InvalidPublicId)
                    .join(state.peers.remove_peer(node_id))
                    .and_then(|_| future::err(()));
                A(report_and_fail)
            } else {
                state.inc_client_count();
                debug!(
                    client.logger(),
                    "connected to peer";
                    "client_count" => state.client_count(),
                );
                let future = client.then(move |res| {
                    state.dec_client_count();
                    res
                });
                B(future)
            }
        });
    spawn_state.spawn(cf);
}

fn trusted_peers_shuffled(config: &Configuration) -> Vec<SocketAddr> {
    let mut peers = config
        .trusted_peers
        .iter()
        .filter_map(|peer| peer.address.to_socketaddr())
        .collect::<Vec<_>>();
    let mut rng = rand::thread_rng();
    peers.shuffle(&mut rng);
    peers
}

#[derive(Clone)]
pub struct BootstrapPeers(BTreeMap<String, Peer>);

impl BootstrapPeers {
    pub fn new() -> Self {
        BootstrapPeers(BTreeMap::new())
    }

    pub fn add_peer(&mut self, peer: Peer) -> usize {
        self.0
            .insert(peer.address().to_string(), peer)
            .map(|_| 0)
            .unwrap_or(1)
    }

    pub fn add_peers(&mut self, peers: &[Peer]) -> usize {
        let mut count = 0;
        for p in peers {
            count += self.add_peer(p.clone());
        }
        count
    }

    pub fn count(&self) -> usize {
        self.0.len()
    }

    pub fn randomly(&self) -> Vec<&Peer> {
        let mut peers = self.0.iter().map(|(_, peer)| peer).collect::<Vec<_>>();
        let mut rng = rand::thread_rng();
        peers.shuffle(&mut rng);
        peers
    }
}

/// Try to get sufficient peers to do a netboot from
fn netboot_peers(config: &Configuration, logger: &Logger) -> BootstrapPeers {
    let mut peers = BootstrapPeers::new();

    // extract the trusted peers from the config
    let trusted_peers = config
        .trusted_peers
        .iter()
        .filter_map(|tp| tp.address.to_socketaddr().map(|sa| Peer::new(sa.clone())))
        .collect::<Vec<_>>();
    if config.bootstrap_from_trusted_peers {
        let _: usize = peers.add_peers(&trusted_peers);
    } else {
        let mut rng = rand::thread_rng();
        let mut trusted_peers = trusted_peers;
        trusted_peers.shuffle(&mut rng);
        for tpeer in trusted_peers {
            //let peer = Peer::new(peer, Protocol::Grpc);
            let tp_logger = logger.new(o!("peer_addr" => tpeer.address().to_string()));
            let received_peers = bootstrap::peers_from_trusted_peer(&tpeer, tp_logger.clone())
                .unwrap_or_else(|e| {
                    warn!(
                        tp_logger,
                        "failed to retrieve the list of bootstrap peers from trusted peer";
                        "reason" => %e,
                    );
                    vec![tpeer]
                });
            let added = peers.add_peers(&received_peers);
            info!(logger, "adding {} peers from peer", added);

            if peers.count() > 32 {
                break;
            }
        }
    }
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

    if config.skip_bootstrap {
        return Ok(true);
    }

    if config.trusted_peers.is_empty() {
        error!(
            logger,
            "trusted-peers cannot be empty. to avoid bootstrap use 'skip_bootstrap: true'"
        );
        //panic!("use non empty trusted-peers or set skip_bootstrap");
    }

    let mut bootstrapped = false;

    let netboot_peers = netboot_peers(config, logger);

    for peer in netboot_peers.randomly() {
        let logger = logger.new(o!("peer_addr" => peer.address().to_string()));
        let res = bootstrap::bootstrap_from_peer(
            peer,
            blockchain.clone(),
            branch.clone(),
            logger.clone(),
        );
        match res {
            Err(bootstrap::Error::Connect { source: e }) => {
                warn!(logger, "unable to reach peer for initial bootstrap"; "reason" => %e);
            }
            Err(e) => {
                warn!(logger, "initial bootstrap failed"; "error" => ?e);
            }
            Ok(()) => {
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
    hash: HeaderHash,
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
        let peer = Peer::new(address);
        match grpc::fetch_block(&peer, hash, &logger) {
            Err(grpc::FetchBlockError::Connect { source: e }) => {
                warn!(logger, "unable to reach peer for block download"; "reason" => %e);
            }
            Err(e) => {
                warn!(logger, "failed to download block"; "error" => ?e);
            }
            Ok(b) => {
                info!(logger, "genesis block fetched");
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

#[derive(Debug, Error)]
pub enum FetchBlockError {
    #[error("no trusted peers specified")]
    NoTrustedPeers,
    #[error("could not download block hash {block}")]
    CouldNotDownloadBlock { block: HeaderHash },
}
