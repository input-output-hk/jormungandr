//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

pub mod bootstrap;
mod client;
mod convert;
mod grpc;
pub mod p2p;
mod service;
mod subscription;

use self::convert::Encode;

use futures::{future, prelude::*};
use poldercast::Address;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use thiserror::Error;
use tokio::time;
use tokio_util::sync::CancellationToken;

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

    // How many concurrent requests are permitted per server connection
    pub const SERVER_REQUESTS: usize = 256;
}

mod keepalive_durations {
    use std::time::Duration;

    // TCP level keepalive duration for client and server connections
    pub const TCP: Duration = Duration::from_secs(60);

    // HTTP/2 keepalive for client connections
    pub const HTTP2: Duration = Duration::from_secs(120);
}

mod security_params {
    pub const NONCE_LEN: usize = 32;
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
use chain_network::data::NodeKeyPair;
use poldercast::StrikeReason;
use rand::seq::SliceRandom;
use slog::Logger;
use tonic::transport;

use std::collections::BTreeMap;
use std::error;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

pub use self::bootstrap::Error as BootstrapError;
use crate::stats_counter::StatsCounter;

#[derive(Debug)]
pub struct ListenError {
    cause: transport::Error,
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
    block0_hash: HeaderHash,
    config: Configuration,
    stats_counter: StatsCounter,
    topology: P2pTopology,
    peers: Peers,
    keypair: NodeKeyPair,
    logger: Logger,
}

pub type GlobalStateR = Arc<GlobalState>;

impl GlobalState {
    /// the network global state
    pub fn new(
        block0_hash: HeaderHash,
        config: Configuration,
        stats_counter: StatsCounter,
        logger: Logger,
    ) -> Self {
        let peers = Peers::new(config.max_connections, logger.clone());

        let mut rng_seed = [0; 32];
        rand::thread_rng().fill(&mut rng_seed);
        let mut prng = ChaChaRng::from_seed(rng_seed);

        let keypair = NodeKeyPair::generate(&mut prng);

        let topology = P2pTopology::new(
            &config,
            logger.new(o!(log::KEY_SUB_TASK => "poldercast")),
            prng,
        );

        GlobalState {
            block0_hash,
            config,
            stats_counter,
            topology,
            peers,
            keypair,
            logger,
        }
    }

    fn logger(&self) -> &Logger {
        &self.logger
    }

    pub fn node_address(&self) -> Option<&Address> {
        self.config.profile.address()
    }

    pub fn topology(&self) -> &P2pTopology {
        &self.topology
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(f);
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
        let count = self.stats_counter.peer_connected_cnt().saturating_add(1);
        if count > self.config.max_inbound_connections {
            count - self.config.max_inbound_connections
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
    fn new(global: GlobalStateR, peer: &Peer, logger: Logger) -> Self {
        ConnectionState {
            timeout: peer.timeout,
            connection: peer.connection,
            logger,
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
    pub global_state: GlobalStateR,
    pub input: MessageQueue<NetworkMsg>,
    pub channels: Channels,
}

pub async fn start(service_info: TokioServiceInfo, params: TaskParams) {
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let input = params.input;
    let channels = params.channels;
    let global_state = params.global_state;

    // open the port for listening/accepting other peers to connect too
    let listen_state = global_state.clone();
    let listen_channels = channels.clone();
    let listener = async move {
        if let Some(listen) = listen_state.config.listen() {
            match listen.protocol {
                Protocol::Grpc => {
                    grpc::run_listen_socket(&listen, listen_state, listen_channels)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                reason = %e,
                                "failed to listen for P2P connections at {}", listen.connection
                            );
                        });
                }
                Protocol::Ntt => unimplemented!(),
            }
        }
    };

    service_info.spawn(
        "gossip",
        start_gossiping(global_state.clone(), channels.clone()),
    );

    let handle_cmds = handle_network_input(input, global_state.clone(), channels.clone());

    let reset_state = global_state.clone();

    if let Some(interval) = global_state.config.topology_force_reset_interval {
        service_info.run_periodic("force reset topology", interval, move || {
            let state = reset_state.clone();
            async move { state.topology.force_reset_layers().await }
        });
    }

    let gossip = async {
        let mut gossip_interval = time::interval(global_state.config.gossip_interval);
        loop {
            gossip_interval.tick().await;
            send_gossip(global_state.clone(), channels.clone()).await
        }
    };

    future::join3(listener, handle_cmds, gossip).await;
}

async fn handle_network_input(
    mut input: MessageQueue<NetworkMsg>,
    state: GlobalStateR,
    channels: Channels,
) {
    while let Some(msg) = input.next().await {
        match msg {
            NetworkMsg::Propagate(msg) => {
                handle_propagation_msg(msg, state.clone(), channels.clone()).await;
            }
            NetworkMsg::GetBlocks(block_ids) => state.peers.fetch_blocks(block_ids.encode()).await,
            NetworkMsg::GetNextBlock(node_id, block_id) => {
                state
                    .peers
                    .solicit_blocks(node_id, Box::new([block_id.encode()]))
                    .await;
            }
            NetworkMsg::PullHeaders {
                node_address,
                from,
                to,
            } => {
                let from: Vec<_> = from.into();
                state
                    .peers
                    .pull_headers(node_address, from.encode(), to.encode())
                    .await;
            }
            NetworkMsg::PeerInfo(reply) => {
                state.peers.infos().map(|infos| reply.reply_ok(infos)).await;
            }
        }
    }
}

async fn handle_propagation_msg(msg: PropagateMsg, state: GlobalStateR, channels: Channels) {
    let prop_state = state.clone();
    let propagate_res = match &msg {
        PropagateMsg::Block(header) => {
            debug!(state.logger(), "block to propagate"; "hash" => %header.hash());
            let header = header.encode();
            let view = state
                .topology
                .view(poldercast::Selection::Topic {
                    topic: p2p::topic::BLOCKS,
                })
                .await;
            prop_state.peers.propagate_block(view.peers, header).await
        }
        PropagateMsg::Fragment(fragment) => {
            debug!(state.logger(), "fragment to propagate"; "hash" => %fragment.hash());
            let fragment = fragment.encode();
            let view = state
                .topology
                .view(poldercast::Selection::Topic {
                    topic: p2p::topic::MESSAGES,
                })
                .await;
            prop_state
                .peers
                .propagate_fragment(view.peers, fragment)
                .await
        }
    };
    // If any nodes selected for propagation are not in the
    // active subscriptions map, connect to them and deliver
    // the item.
    if let Err(unreached_nodes) = propagate_res {
        debug!(
            state.logger(),
            "will try to connect to the peers not immediately reachable for propagation: {:?}",
            unreached_nodes,
        );
        for node in unreached_nodes {
            let mut options = p2p::comm::ConnectOptions::default();
            match &msg {
                PropagateMsg::Block(header) => {
                    options.pending_block_announcement = Some(header.encode());
                }
                PropagateMsg::Fragment(fragment) => {
                    options.pending_fragment = Some(fragment.encode());
                }
            };
            connect_and_propagate(node, state.clone(), channels.clone(), options);
        }
    }
}

async fn start_gossiping(state: GlobalStateR, channels: Channels) {
    let config = &state.config;
    let topology = &state.topology;
    let logger = state.logger().new(o!(log::KEY_SUB_TASK => "start_gossip"));
    // inject the trusted peers as initial gossips, this will make the node
    // gossip with them at least at the beginning

    for tp in config.trusted_peers.iter() {
        topology
            .accept_gossips(tp.address.clone(), {
                let mut builder = poldercast::NodeProfileBuilder::new();
                builder.address(tp.address.clone());
                if let Some(id) = tp.legacy_node_id {
                    builder.id(id);
                }
                p2p::Gossips::from(vec![p2p::Gossip::from(builder.build())])
            })
            .await;
    }
    let view = topology.view(poldercast::Selection::Any).await;
    let peers: Vec<p2p::Address> = view.peers;
    debug!(logger, "sending gossip to {} peers", peers.len());
    for address in peers {
        let gossips = topology.initiate_gossips(address.clone()).await;
        let propagate_res = state
            .peers
            .propagate_gossip_to(address.clone(), Gossip::from(gossips))
            .await;
        if let Err(gossip) = propagate_res {
            let options = p2p::comm::ConnectOptions {
                pending_gossip: Some(gossip),
                ..Default::default()
            };
            connect_and_propagate(address, state.clone(), channels.clone(), options);
        }
    }
}

async fn send_gossip(state: GlobalStateR, channels: Channels) {
    let topology = &state.topology;
    let logger = state.logger().new(o!(log::KEY_SUB_TASK => "send_gossip"));
    let view = topology.view(poldercast::Selection::Any).await;
    let peers = view.peers;
    debug!(logger, "sending gossip to {} peers", peers.len());
    for address in peers {
        let state_prop = state.clone();
        let state_err = state.clone();
        let channels_err = channels.clone();
        let gossips = topology.initiate_gossips(address.clone()).await;
        let res = state_prop
            .peers
            .propagate_gossip_to(address.clone(), Gossip::from(gossips))
            .await;
        if let Err(gossip) = res {
            let options = p2p::comm::ConnectOptions {
                pending_gossip: Some(gossip),
                ..Default::default()
            };
            connect_and_propagate(address, state_err, channels_err, options);
        }
    }
}

fn connect_and_propagate(
    node: p2p::Address,
    state: GlobalStateR,
    channels: Channels,
    mut options: p2p::comm::ConnectOptions,
) {
    let addr = match node.to_socket_addr() {
        Some(addr) => addr,
        None => {
            debug!(
                state.logger(),
                "ignoring P2P node without an IP address" ;
                "peer" => %node
            );
            return;
        }
    };
    options.evict_clients = state.num_clients_to_bump();
    if Some(&node) == state.node_address() {
        error!(state.logger(), "topology tells the node to connect to itself, ignoring"; "peer" => %node);
        return;
    }
    let peer = Peer::new(addr);
    let conn_logger = state.logger().new(o!("peer" => node.to_string()));
    let conn_state = ConnectionState::new(state.clone(), &peer, conn_logger.clone());
    info!(conn_logger, "connecting to peer");
    let (handle, connecting) = client::connect(conn_state, channels);
    let spawn_state = state.clone();
    let cf = async move {
        state
            .peers
            .add_connecting(node.clone(), handle, options)
            .await;
        match connecting.await {
            Err(e) => {
                let benign = match e {
                    ConnectError::Transport(e) => {
                        info!(conn_logger, "gRPC connection to peer failed"; "reason" => %e);
                        false
                    }
                    ConnectError::Handshake(e) => {
                        info!(conn_logger, "protocol handshake with peer failed"; "reason" => %e);
                        false
                    }
                    ConnectError::Canceled => {
                        debug!(conn_logger, "connection to peer has been canceled");
                        true
                    }
                    _ => {
                        info!(conn_logger, "connection to peer failed"; "error" => ?e);
                        false
                    }
                };
                if !benign {
                    future::join(
                        state
                            .topology
                            .report_node(node.clone(), StrikeReason::CannotConnect),
                        state.peers.remove_peer(node.clone()),
                    )
                    .await;
                }
            }
            Ok(client) => {
                state.inc_client_count();
                debug!(
                    client.logger(),
                    "connected to peer";
                    "client_count" => state.client_count(),
                );
                client.await;
                state.dec_client_count();
            }
        }
    };
    spawn_state.spawn(cf);
}

fn trusted_peers_shuffled(config: &Configuration) -> Vec<SocketAddr> {
    let mut peers = config
        .trusted_peers
        .iter()
        .filter_map(|peer| peer.address.to_socket_addr())
        .collect::<Vec<_>>();
    let mut rng = rand::thread_rng();
    peers.shuffle(&mut rng);
    peers
}

#[derive(Clone)]
pub struct BootstrapPeers(BTreeMap<String, Peer>);

impl Default for BootstrapPeers {
    fn default() -> Self {
        Self::new()
    }
}

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
async fn netboot_peers(config: &Configuration, logger: &Logger) -> BootstrapPeers {
    let mut peers = BootstrapPeers::new();

    // extract the trusted peers from the config
    let trusted_peers = config
        .trusted_peers
        .iter()
        .filter_map(|tp| tp.address.to_socket_addr().map(Peer::new))
        .collect::<Vec<_>>();
    if config.bootstrap_from_trusted_peers {
        let _: usize = peers.add_peers(&trusted_peers);
    } else {
        let mut rng = rand::rngs::OsRng;
        let mut trusted_peers = trusted_peers;
        trusted_peers.shuffle(&mut rng);
        for tpeer in trusted_peers {
            // let peer = Peer::new(peer, Protocol::Grpc);
            let tp_logger = logger.new(o!("peer_addr" => tpeer.address().to_string()));
            let received_peers = bootstrap::peers_from_trusted_peer(&tpeer, tp_logger.clone())
                .await
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

pub async fn bootstrap(
    config: &Configuration,
    blockchain: NewBlockchain,
    branch: Tip,
    cancellation_token: CancellationToken,
    logger: &Logger,
) -> Result<bool, bootstrap::Error> {
    use futures::future::{select, Either, FutureExt};

    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }

    if config.skip_bootstrap {
        return Ok(true);
    }

    if config.trusted_peers.is_empty() {
        return Err(bootstrap::Error::EmptyTrustedPeers);
    }

    let mut bootstrapped = false;

    let (netboot_peers, _) = match select(
        netboot_peers(config, logger).boxed(),
        cancellation_token.cancelled().boxed(),
    )
    .await
    {
        Either::Left(result) => result,
        Either::Right(((), _)) => return Err(bootstrap::Error::Interrupted),
    };

    for peer in netboot_peers.randomly() {
        let logger = logger.new(o!("peer_addr" => peer.address().to_string()));
        let res = bootstrap::bootstrap_from_peer(
            peer,
            blockchain.clone(),
            branch.clone(),
            cancellation_token.clone(),
            logger.clone(),
        )
        .await;

        match res {
            Err(bootstrap::Error::Connect(e)) => {
                warn!(logger, "unable to reach peer for initial bootstrap"; "reason" => %e);
            }
            Err(bootstrap::Error::Interrupted) => {
                warn!(logger, "the bootstrap process was interrupted");
                return Err(bootstrap::Error::Interrupted);
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

    blockchain
        .gc(branch.get_ref().await)
        .await
        .map_err(bootstrap::Error::GcFailed)?;

    Ok(bootstrapped)
}

/// Queries the trusted peers for a block identified with the hash.
/// The calling thread is blocked until the block is retrieved.
/// This function is called during blockchain initialization
/// to retrieve the genesis block.
pub async fn fetch_block(
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
        match grpc::fetch_block(&peer, hash, &logger).await {
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
