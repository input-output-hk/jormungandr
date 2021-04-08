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
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
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
use self::p2p::{comm::Peers, NodeId, P2pTopology};
use crate::blockcfg::{Block, HeaderHash};
use crate::blockchain::{Blockchain as NewBlockchain, Tip};
use crate::intercom::{BlockMsg, ClientMsg, NetworkMsg, PropagateMsg, TransactionMsg};
use crate::settings::start::network::{Configuration, Peer, Protocol};
use crate::utils::{
    async_msg::{MessageBox, MessageQueue},
    task::TokioServiceInfo,
};
use chain_network::data::gossip::Gossip;
use chain_network::data::NodeKeyPair;
use rand::seq::SliceRandom;
use tonic::transport;
use tracing::{span, Level, Span};
use tracing_futures::Instrument;

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
    span: Span,
}

pub type GlobalStateR = Arc<GlobalState>;

impl GlobalState {
    /// the network global state
    pub fn new(
        block0_hash: HeaderHash,
        config: Configuration,
        stats_counter: StatsCounter,
        span: Span,
    ) -> Self {
        let peers = Peers::new(
            config.max_connections,
            span!(parent: &span, Level::TRACE, "peers"),
        );

        let mut rng_seed = [0; 32];
        rand::thread_rng().fill(&mut rng_seed);
        let mut prng = ChaChaRng::from_seed(rng_seed);

        let keypair = NodeKeyPair::generate(&mut prng);

        let topology = P2pTopology::new(&config);

        GlobalState {
            block0_hash,
            config,
            stats_counter,
            topology,
            peers,
            keypair,
            span,
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn node_address(&self) -> Option<SocketAddr> {
        self.config.public_address
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

    pub span: Span,
}

impl ConnectionState {
    fn new(global: GlobalStateR, peer: &Peer, span: Span) -> Self {
        ConnectionState {
            timeout: peer.timeout,
            connection: peer.connection,
            span,
            global,
        }
    }

    fn peer(&self) -> Peer {
        Peer::with_timeout(self.connection, self.timeout)
    }

    fn span(&self) -> &Span {
        &self.span
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
    async {
        let prop_state = state.clone();
        let propagate_res = match &msg {
            PropagateMsg::Block(header) => {
                tracing::debug!(hash = %header.hash(), "block to propagate");
                let header = header.encode();
                let view = state
                    .topology
                    .view(poldercast::layer::Selection::Topic {
                        topic: p2p::topic::BLOCKS,
                    })
                    .await;
                prop_state
                    .peers
                    .propagate_block(
                        view.peers
                            .into_iter()
                            .map(|profile| (profile.address(), profile.id()))
                            .collect(),
                        header,
                    )
                    .await
            }
            PropagateMsg::Fragment(fragment) => {
                tracing::debug!(hash = %fragment.hash(), "fragment to propagate");
                let fragment = fragment.encode();
                let view = state
                    .topology
                    .view(poldercast::layer::Selection::Topic {
                        topic: p2p::topic::MESSAGES,
                    })
                    .await;

                prop_state
                    .peers
                    .propagate_fragment(
                        view.peers
                            .into_iter()
                            .map(|profile| (profile.address(), profile.id()))
                            .collect(),
                        fragment,
                    )
                    .await
            }
        };
        // If any nodes selected for propagation are not in the
        // active subscriptions map, connect to them and deliver
        // the item.
        if let Err(unreached_nodes) = propagate_res {
            tracing::debug!(
                "will try to connect to the peers not immediately reachable for propagation: {:?}",
                unreached_nodes,
            );
            for (addr, id) in unreached_nodes {
                let mut options = p2p::comm::ConnectOptions::default();
                match &msg {
                    PropagateMsg::Block(header) => {
                        options.pending_block_announcement = Some(header.encode());
                    }
                    PropagateMsg::Fragment(fragment) => {
                        options.pending_fragment = Some(fragment.encode());
                    }
                };
                connect_and_propagate(addr, Some(id), state.clone(), channels.clone(), options);
            }
        }
    }
    .instrument(state.span.clone())
    .await
}

async fn start_gossiping(state: GlobalStateR, channels: Channels) {
    let config = &state.config;
    let span = span!(parent: &state.span, Level::TRACE, "sub_task", kind = "start_gossip");
    async {
        tracing::debug!("connecting to {} peers", config.trusted_peers.len());
        for peer in &config.trusted_peers {
            let gossip = state.topology.initiate_gossips(None).await;
            let options = p2p::comm::ConnectOptions {
                pending_gossip: Some(Gossip::from(gossip)),
                ..Default::default()
            };
            connect_and_propagate(peer.addr, None, state.clone(), channels.clone(), options);
        }
    }
    .instrument(span)
    .await
}

async fn send_gossip(state: GlobalStateR, channels: Channels) {
    let topology = &state.topology;
    let span = span!(parent: &state.span, Level::TRACE, "sub_task", kind = "send_gossip");
    async {
        let view = topology.view(poldercast::layer::Selection::Any).await;
        let peers = view.peers;
        tracing::debug!("sending gossip to {} peers", peers.len());
        for peer in peers {
            let addr = peer.address();
            let id = peer.id();
            let gossips = topology.initiate_gossips(Some(&id)).await;
            let res = state
                .clone()
                .peers
                .propagate_gossip_to(addr, Gossip::from(gossips))
                .await;
            if let Err(gossip) = res {
                let options = p2p::comm::ConnectOptions {
                    pending_gossip: Some(gossip),
                    ..Default::default()
                };
                connect_and_propagate(addr, Some(id), state.clone(), channels.clone(), options);
            }
        }
    }
    .instrument(span)
    .await
}

// node_id should be none only for trusted peer for which we do not know
// the public key
fn connect_and_propagate(
    node_addr: SocketAddr,
    node_id: Option<NodeId>,
    state: GlobalStateR,
    channels: Channels,
    mut options: p2p::comm::ConnectOptions,
) {
    let _enter = state.span.enter();
    options.evict_clients = state.num_clients_to_bump();
    if let Some(self_addr) = state.node_address() {
        if node_addr == self_addr {
            tracing::error!(peer = %node_addr, "topology tells the node to connect to itself, ignoring");
            return;
        }
    }
    drop(_enter);
    let peer = Peer::new(node_addr);
    let conn_span = span!(parent: &state.span, Level::TRACE, "peer", node = %node_addr);
    let _enter = conn_span.enter();
    let conn_state = ConnectionState::new(state.clone(), &peer, conn_span.clone());
    tracing::info!("connecting to peer");
    let (handle, connecting) = client::connect(conn_state, channels);
    let spawn_state = state.clone();
    let cf = async move {
        state.peers.add_connecting(node_addr, handle, options).await;
        match connecting.await {
            Err(e) => {
                let benign = match e {
                    ConnectError::Transport(e) => {
                        tracing::info!(reason = %e, "gRPC connection to peer failed");
                        false
                    }
                    ConnectError::Handshake(e) => {
                        tracing::info!(reason = %e, "protocol handshake with peer failed");
                        false
                    }
                    ConnectError::Canceled => {
                        tracing::debug!("connection to peer has been canceled");
                        true
                    }
                    _ => {
                        tracing::info!(error = ?e, "connection to peer failed");
                        false
                    }
                };
                if !benign {
                    if let Some(id) = node_id {
                        state.topology.report_node(&id).await;
                    }
                    state.peers.remove_peer(node_addr).await;
                }
            }
            Ok(client) => {
                // This enforce processing any pending operation that could
                // have been scheduled on this peer
                state.peers.update_entry(node_addr).await;

                state.inc_client_count();
                if let Some(id) = node_id {
                    state.topology().promote_node(&id).await;
                }
                tracing::debug!(client_count = state.client_count(), "connected to peer");
                client.await;
                state.dec_client_count();
            }
        }
    }
    .instrument(conn_span.clone());
    spawn_state.spawn(cf);
}

fn trusted_peers_shuffled(config: &Configuration) -> Vec<SocketAddr> {
    let mut peers = config
        .trusted_peers
        .iter()
        .map(|peer| peer.addr)
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
async fn netboot_peers(config: &Configuration, parent_span: &Span) -> BootstrapPeers {
    let mut peers = BootstrapPeers::new();

    // extract the trusted peers from the config
    let trusted_peers = config
        .trusted_peers
        .iter()
        .map(|tp| Peer::new(tp.addr))
        .collect::<Vec<_>>();
    if config.bootstrap_from_trusted_peers {
        let _: usize = peers.add_peers(&trusted_peers);
    } else {
        let mut rng = rand::rngs::OsRng;
        let mut trusted_peers = trusted_peers;
        trusted_peers.shuffle(&mut rng);
        for tpeer in trusted_peers {
            let span = span!(
                parent: parent_span,
                Level::TRACE,
                "netboot_peers",
                peer_addr = %tpeer.address().to_string()
            );
            peers = async move {
                let received_peers = bootstrap::peers_from_trusted_peer(&tpeer)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::warn!(
                            reason = %e,
                            "failed to retrieve the list of bootstrap peers from trusted peer"
                        );
                        vec![tpeer]
                    });
                let added = peers.add_peers(&received_peers);
                tracing::info!("adding {} peers from peer", added);
                peers
            }
            .instrument(span)
            .await;

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
    span: &Span,
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
        netboot_peers(config, span).boxed(),
        cancellation_token.cancelled().boxed(),
    )
    .await
    {
        Either::Left(result) => result,
        Either::Right(((), _)) => return Err(bootstrap::Error::Interrupted),
    };

    for peer in netboot_peers.randomly() {
        let span =
            span!(parent: span, Level::TRACE, "bootstrap", peer_addr = %peer.address().to_string());
        let res = bootstrap::bootstrap_from_peer(
            peer,
            blockchain.clone(),
            branch.clone(),
            cancellation_token.clone(),
        )
        .await;

        match res {
            Err(bootstrap::Error::Connect(e)) => {
                async move {
                    tracing::warn!(reason = %e, "unable to reach peer for initial bootstrap");
                }
                .instrument(span)
                .await;
            }
            Err(bootstrap::Error::Interrupted) => {
                async move {
                    tracing::warn!("the bootstrap process was interrupted");
                }
                .instrument(span)
                .await;
                return Err(bootstrap::Error::Interrupted);
            }
            Err(e) => {
                async move {
                    tracing::warn!(error = ?e, "initial bootstrap failed");
                }
                .instrument(span)
                .await;
            }
            Ok(()) => {
                async move {
                    tracing::info!("initial bootstrap completed");
                }
                .instrument(span)
                .await;

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
) -> Result<Block, FetchBlockError> {
    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }

    if config.trusted_peers.is_empty() {
        return Err(FetchBlockError::NoTrustedPeers);
    }

    let mut block = None;

    let span = span!(Level::TRACE, "fetch_block", block = %hash.to_string());
    async {
        for address in trusted_peers_shuffled(&config) {
            let peer_span = span!(Level::TRACE, "peer_address", address = %address.to_string());
            let peer = Peer::new(address);
            match grpc::fetch_block(&peer, hash)
                .instrument(peer_span.clone())
                .await
            {
                Err(grpc::FetchBlockError::Connect { source: e }) => {
                    async {
                        tracing::warn!(reason = %e, "unable to reach peer for block download");
                    }
                    .instrument(peer_span)
                    .await
                }
                Err(e) => {
                    async {
                        tracing::warn!(error = ?e, "failed to download block");
                    }
                    .instrument(peer_span)
                    .await
                }
                Ok(b) => {
                    async {
                        tracing::info!("genesis block fetched");
                    }
                    .instrument(peer_span)
                    .await;

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
    .instrument(span)
    .await
}

#[derive(Debug, Error)]
pub enum FetchBlockError {
    #[error("no trusted peers specified")]
    NoTrustedPeers,
    #[error("could not download block hash {block}")]
    CouldNotDownloadBlock { block: HeaderHash },
}
