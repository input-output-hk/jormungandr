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
use thiserror::Error;
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

pub use self::bootstrap::Error as BootstrapError;
use self::{client::ConnectError, p2p::comm::Peers};
use crate::{
    blockcfg::{Block, HeaderHash},
    blockchain::{Blockchain as NewBlockchain, Tip},
    intercom::{BlockMsg, ClientMsg, NetworkMsg, PropagateMsg, TopologyMsg, TransactionMsg},
    metrics::Metrics,
    settings::start::network::{Configuration, Peer, Protocol},
    topology::{self, NodeId},
    utils::async_msg::{MessageBox, MessageQueue},
};
use chain_network::data::NodeKeyPair;
use rand::seq::SliceRandom;
use std::{
    collections::HashSet,
    error, fmt,
    iter::FromIterator,
    net::SocketAddr,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tonic::transport;
use tracing::{instrument, span, Level, Span};
use tracing_futures::Instrument;

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

#[derive(Error, Debug)]
enum PropagateError {
    #[error("Error sending message to task due to {0}")]
    InternalCommSend(#[from] futures::channel::mpsc::SendError),
    #[error("Error receving message from task due to {0}")]
    InternalCommRecv(#[from] crate::intercom::Error),
}

type Connection = SocketAddr;

/// all the different channels the network may need to talk to
pub struct Channels {
    pub client_box: MessageBox<ClientMsg>,
    pub transaction_box: MessageBox<TransactionMsg>,
    pub block_box: MessageBox<BlockMsg>,
    pub topology_box: MessageBox<TopologyMsg>,
}

impl Clone for Channels {
    fn clone(&self) -> Self {
        Channels {
            client_box: self.client_box.clone(),
            transaction_box: self.transaction_box.clone(),
            block_box: self.block_box.clone(),
            topology_box: self.topology_box.clone(),
        }
    }
}

/// Global state shared between all network tasks.
pub struct GlobalState {
    block0_hash: HeaderHash,
    config: Configuration,
    peers: Peers,
    keypair: NodeKeyPair,
    span: Span,

    connected_count: AtomicUsize,
}

pub type GlobalStateR = Arc<GlobalState>;

impl GlobalState {
    /// the network global state
    pub fn new(
        block0_hash: HeaderHash,
        config: Configuration,
        stats_counter: Metrics,
        span: Span,
    ) -> Self {
        let peers = Peers::new(config.max_connections, stats_counter);

        //TODO: move this to a secure enclave
        let keypair =
            NodeKeyPair::from(<chain_crypto::SecretKey<_>>::from(config.node_key.clone()));

        GlobalState {
            block0_hash,
            config,
            peers,
            keypair,
            span,
            connected_count: AtomicUsize::new(0),
        }
    }

    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn node_address(&self) -> Option<SocketAddr> {
        self.config.public_address
    }

    pub fn spawn<F>(&self, f: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        tokio::spawn(f);
    }

    fn inc_client_count(&self) {
        self.connected_count.fetch_add(1, Ordering::AcqRel);
    }

    fn dec_client_count(&self) {
        self.connected_count.fetch_sub(1, Ordering::AcqRel);
    }

    fn client_count(&self) -> usize {
        self.connected_count.load(Ordering::Acquire)
    }

    // How many client connections to bump when a new one is about to be
    // established
    fn num_clients_to_bump(&self) -> usize {
        let count = self.client_count().saturating_add(1);
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
    pub watch: crate::watch_client::WatchClient,
}

pub async fn start(params: TaskParams) {
    // TODO: the node needs to be saved/loaded
    //
    // * the ID needs to be consistent between restart;
    let input = params.input;
    let channels = params.channels;
    let global_state = params.global_state;
    let watch = params.watch;

    // open the port for listening/accepting other peers to connect too
    let listen_state = global_state.clone();
    let listen_channels = channels.clone();
    let listener = async move {
        if let Some(listen) = listen_state.config.listen() {
            match listen.protocol {
                Protocol::Grpc => {
                    grpc::run_listen_socket(
                        &listen,
                        listen_state,
                        listen_channels,
                        watch.into_server(),
                    )
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

    let handle_cmds = handle_network_input(input, global_state.clone(), channels.clone());
    future::join(listener, handle_cmds).await;
}

async fn handle_network_input(
    mut input: MessageQueue<NetworkMsg>,
    state: GlobalStateR,
    channels: Channels,
) {
    while let Some(msg) = input.next().await {
        tracing::trace!("handling new network task item");
        match msg {
            NetworkMsg::Propagate(msg) => {
                handle_propagation_msg(*msg, state.clone(), channels.clone())
                    .await
                    .unwrap_or_else(|e| tracing::error!("Error while propagating message: {}", e));
            }
            NetworkMsg::GetBlocks(block_ids) => {
                state.peers.solicit_blocks_any(block_ids.encode()).await
            }
            NetworkMsg::GetNextBlock(node_id, block_id) => {
                state
                    .peers
                    .solicit_blocks_peer(&node_id, Box::new([block_id.encode()]))
                    .await;
            }
            NetworkMsg::PullHeaders { node_id, from, to } => {
                let from: Vec<_> = from.into();
                state
                    .peers
                    .pull_headers(&node_id, from.encode(), to.encode())
                    .await;
            }
            NetworkMsg::PeerInfo(reply) => {
                state.peers.infos().map(|infos| reply.reply_ok(infos)).await;
            }
        };
        tracing::trace!("item handling finished");
    }
}

// propagate message to every peer and return the ones that we could not contact
async fn propagate_message<F, Fut, E, T>(
    f: F,
    sel: poldercast::layer::Selection,
    arg: T,
    mbox: &mut MessageBox<TopologyMsg>,
) -> Result<Vec<topology::Peer>, PropagateError>
where
    T: Clone,
    F: Fn(NodeId, T) -> Fut,
    Fut: Future<Output = Result<(), E>>,
{
    let (reply_handle, reply_future) = crate::intercom::unary_reply();
    mbox.send(TopologyMsg::View(sel, reply_handle)).await?;
    let peers = reply_future.await.map(|view| view.peers)?;

    // FIXME: this is a workaround because we need to know also the id of the nodes that failed to connect,
    // it should not be less efficient, just less clean. Remove this once we decided what to do with peers
    // and ids.
    let mut res = Vec::new();
    for peer in peers {
        if f(peer.id(), arg.clone())
            .instrument(span!(Level::DEBUG, "p2p_comm", peer = %peer.address(), id = %peer.id()))
            .await
            .is_err()
        {
            res.push(peer);
        }
    }
    Ok(res)
}

#[instrument(level = "debug", skip_all, fields(addr, hash, id))]
async fn handle_propagation_msg(
    msg: PropagateMsg,
    state: GlobalStateR,
    mut channels: Channels,
) -> Result<(), PropagateError> {
    use poldercast::layer::Selection;
    let prop_state = state.clone();
    let unreached_nodes = match &msg {
        PropagateMsg::Block(header) => {
            Span::current().record("hash", format_args!("{}", header.description()));
            tracing::debug!("received new block to propagate");
            let header = header.encode();
            propagate_message(
                |id, header| prop_state.peers.propagate_block(id, header),
                Selection::Topic {
                    topic: crate::topology::topic::BLOCKS,
                },
                header,
                &mut channels.topology_box,
            )
            .await?
        }
        PropagateMsg::Fragment(fragment) => {
            Span::current().record("hash", format_args!("{}", fragment.hash()));
            tracing::debug!(hash = %fragment.hash(), "fragment to propagate");
            let fragment = fragment.encode();
            propagate_message(
                |id, fragment| prop_state.peers.propagate_fragment(id, fragment),
                Selection::Topic {
                    topic: crate::topology::topic::MESSAGES,
                },
                fragment,
                &mut channels.topology_box,
            )
            .await?
        }
        PropagateMsg::Gossip(peer, gossips) => {
            Span::current().record("addr", peer.address().to_string().as_str());
            Span::current().record("id", peer.address().to_string().as_str());
            tracing::debug!("gossip to propagate");
            let gossip = gossips.encode();
            match prop_state
                .peers
                .propagate_gossip_to(peer.id(), gossip)
                .await
            {
                Err(_) => vec![peer.clone()],
                Ok(_) => Vec::new(),
            }
        }
    };
    // If any nodes selected for propagation are not in the
    // active subscriptions map, connect to them and deliver
    // the item.
    if !unreached_nodes.is_empty() {
        tracing::debug!(
            "will try to connect to the peers not immediately reachable for propagation: {:?}",
            unreached_nodes,
        );
        for peer in unreached_nodes {
            let mut options = p2p::comm::ConnectOptions::default();
            match &msg {
                PropagateMsg::Block(header) => {
                    options.pending_block_announcement = Some(header.encode());
                }
                PropagateMsg::Fragment(fragment) => {
                    options.pending_fragment = Some(fragment.encode());
                }
                PropagateMsg::Gossip(_, gossip) => {
                    options.pending_gossip = Some(gossip.encode());
                }
            };
            let (addr, id) = (peer.address(), peer.id());
            connect_and_propagate(addr, id, state.clone(), channels.clone(), options);
        }
    }
    Ok(())
}

// node_id should be missing only for trusted peer for which we do not know
// the public key
fn connect_and_propagate(
    addr: SocketAddr,
    id: NodeId,
    state: GlobalStateR,
    mut channels: Channels,
    mut options: p2p::comm::ConnectOptions,
) {
    let _enter = state.span.enter();
    options.evict_clients = state.num_clients_to_bump();
    if let Some(self_addr) = state.node_address() {
        if addr == self_addr {
            tracing::error!(peer = %addr, "topology tells the node to connect to itself, ignoring");
            return;
        }
    }
    drop(_enter);
    let peer = Peer::new(addr);
    let conn_span = span!(parent: &state.span, Level::DEBUG, "client", %addr, %id);
    let spawn_state = state.clone();
    let cf = async move {
        let conn_state = ConnectionState::new(state.clone(), &peer, Span::current());
        tracing::info!("connecting to peer");
        let (handle, connecting) = client::connect(conn_state, channels.clone(), id);
        state.peers.add_connecting(id, addr, handle, options).await;
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
                    channels
                        .topology_box
                        .send(TopologyMsg::DemotePeer(id))
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("Error sending message to topology task: {}", e)
                        });
                    state.peers.remove_peer(&id).await;
                }
            }
            Ok(client) => {
                // This enforce processing any pending operation that could
                // have been scheduled on this peer
                state.peers.update_entry(id).await;

                state.inc_client_count();

                channels
                    .topology_box
                    .send(TopologyMsg::PromotePeer(id))
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("Error sending message to topology task: {}", e)
                    });
                tracing::debug!(client_count = state.client_count(), "connected to peer");
                client.await;
                state.dec_client_count();
            }
        }
    }
    .instrument(conn_span);
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
pub struct BootstrapPeers {
    // Peers we will try to bootstrap from
    pub bootstrap_peers: Vec<topology::Peer>,
    // Peers we collected while getting bootstrap_peers, which we will not
    // use to bootstrap but rather inject in the topology at startup
    pub topology_peers: Vec<topology::Peer>,
}

/// Try to get sufficient peers to do a netboot from
async fn netboot_peers(config: &Configuration, parent_span: &Span) -> BootstrapPeers {
    let trusted_peers_addrs = config
        .trusted_peers
        .iter()
        .map(|peer| peer.addr)
        .collect::<Vec<_>>();
    let mut peers = HashSet::new();
    for tpeer in &trusted_peers_addrs {
        let span = span!(
            parent: parent_span,
            Level::DEBUG,
            "netboot_peers",
            peer_addr = %tpeer.to_string()
        );
        let received_peers = async move {
            let res = bootstrap::peers_from_trusted_peer(&Peer::new(*tpeer))
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        reason = %e,
                        "failed to retrieve the list of bootstrap peers from trusted peer"
                    );
                    Vec::new()
                });
            tracing::info!("adding {} peers from peer", res.len());
            res
        }
        .instrument(span)
        .await;
        peers.extend(received_peers);
    }

    let (bootstrap_peers, topology_peers) = if config.bootstrap_from_trusted_peers {
        peers
            .into_iter()
            .partition(|peer| trusted_peers_addrs.contains(&peer.address()))
    } else {
        (Vec::from_iter(peers), Vec::new())
    };

    BootstrapPeers {
        bootstrap_peers,
        topology_peers,
    }
}

pub struct NetworkBootstrapResult {
    pub initial_peers: Vec<topology::Peer>,
    pub bootstrapped: bool,
}

pub async fn bootstrap(
    config: &Configuration,
    blockchain: NewBlockchain,
    branch: Tip,
    cancellation_token: CancellationToken,
    span: &Span,
) -> Result<NetworkBootstrapResult, bootstrap::Error> {
    use futures::future::{select, Either, FutureExt};

    if config.protocol != Protocol::Grpc {
        unimplemented!()
    }

    if config.skip_bootstrap {
        return Ok(NetworkBootstrapResult {
            initial_peers: Vec::new(),
            bootstrapped: true,
        });
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

    let BootstrapPeers {
        mut bootstrap_peers,
        topology_peers,
    } = netboot_peers;
    let mut rng = rand::thread_rng();
    bootstrap_peers.shuffle(&mut rng);

    for peer in &bootstrap_peers {
        let span =
            span!(parent: span, Level::DEBUG, "bootstrap", peer_addr = %peer.address().to_string());
        let res = bootstrap::bootstrap_from_peer(
            &Peer::new(peer.address()),
            blockchain.clone(),
            branch.clone(),
            cancellation_token.clone(),
        )
        .instrument(span.clone())
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
        .map_err(|e| bootstrap::Error::GcFailed(Box::new(e)))?;

    Ok(NetworkBootstrapResult {
        initial_peers: bootstrap_peers
            .into_iter()
            .chain(topology_peers.into_iter())
            .collect(),
        bootstrapped,
    })
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

    let span = span!(Level::DEBUG, "fetch_block", block = %hash.to_string());
    async {
        for address in trusted_peers_shuffled(config) {
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
