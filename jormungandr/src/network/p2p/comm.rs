mod peer_map;
use super::Address;
use crate::network::{client::ConnectHandle, security_params::NONCE_LEN};
use chain_network::data::block::{BlockEvent, ChainPullRequest};
use chain_network::data::{BlockId, BlockIds, Fragment, Gossip, Header, NodeId};
use futures::channel::mpsc;
use futures::lock::{Mutex, MutexLockFuture};
use futures::prelude::*;
use futures::stream;
use peer_map::{CommStatus, PeerMap};
use rand::Rng;
use tracing::Span;

use std::collections::HashSet;
use std::fmt;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
use tracing_futures::Instrument;

// Buffer size determines the number of stream items pending processing that
// can be buffered before back pressure is applied to the inbound half of
// a gRPC subscription stream.
const BUFFER_LEN: usize = 8;

#[derive(Debug)]
pub struct PropagateError<T> {
    kind: ErrorKind,
    item: T,
}

impl<T> PropagateError<T> {
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn into_item(self) -> T {
        self.item
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    NotSubscribed,
    SubscriptionClosed,
    StreamOverflow,
    Unexpected,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ErrorKind::*;
        let msg = match self {
            NotSubscribed => "not subscribed",
            SubscriptionClosed => "subscription has been closed",
            StreamOverflow => "too many items queued",
            Unexpected => "unexpected error (should never occur?)",
        };
        f.write_str(msg)
    }
}

/// Stream used as the outbound half of a subscription stream.
pub struct OutboundSubscription<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> Stream for OutboundSubscription<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

type BlockEventAnnounceStream = stream::Map<OutboundSubscription<Header>, fn(Header) -> BlockEvent>;

type BlockEventSolicitStream =
    stream::Map<OutboundSubscription<BlockIds>, fn(BlockIds) -> BlockEvent>;

type BlockEventMissingStream =
    stream::Map<OutboundSubscription<ChainPullRequest>, fn(ChainPullRequest) -> BlockEvent>;

pub type BlockEventSubscription = stream::Select<
    BlockEventAnnounceStream,
    stream::Select<BlockEventSolicitStream, BlockEventMissingStream>,
>;

pub type FragmentSubscription = OutboundSubscription<Fragment>;

pub type GossipSubscription = OutboundSubscription<Gossip>;

/// Handle used by the per-peer communication tasks to produce an outbound
/// subscription stream towards the peer.
pub struct CommHandle<T> {
    state: SubscriptionState<T>,
    direction: SubscriptionDirection,
}

/// Indicates whether this subscription belongs to a client or a server
/// connection.
#[derive(Copy, Clone)]
pub enum SubscriptionDirection {
    Client,
    Server,
}

impl<T> Default for CommHandle<T> {
    fn default() -> Self {
        CommHandle {
            state: SubscriptionState::NotSubscribed,
            direction: SubscriptionDirection::Server,
        }
    }
}

impl<T> CommHandle<T> {
    /// Creates a handle with the `Client` direction and an item waiting to be sent,
    /// in expectation for a subscription to be established.
    pub fn client_pending(item: T) -> Self {
        CommHandle {
            state: SubscriptionState::Pending(item),
            direction: SubscriptionDirection::Client,
        }
    }

    pub fn direction(&self) -> SubscriptionDirection {
        self.direction
    }

    pub fn is_client(&self) -> bool {
        matches!(self.direction, SubscriptionDirection::Client)
    }

    pub fn clear_pending(&mut self) {
        if let SubscriptionState::Pending(_) = self.state {
            self.state = SubscriptionState::NotSubscribed;
        }
    }

    /// Updates this handle with the subscription state from another
    /// handle. This happens when another connection is established
    /// to the same peer. This method is used instead of replacing
    /// the handle to send a potential pending item over the new subscription.
    pub fn update(&mut self, newer: CommHandle<T>) {
        self.direction = newer.direction;
        if let SubscriptionState::Pending(item) = mem::replace(&mut self.state, newer.state) {
            // If there is an error sending the pending item,
            // it is silently dropped. Logging infrastructure to debug
            // this would be nice.
            let _ = self.try_send(item);
        }
    }

    /// Returns a stream to use as an outbound half of the
    /// subscription stream.
    ///
    /// If this method is called again on the same handle,
    /// the previous subscription is closed and its stream is terminated.
    pub fn subscribe(&mut self) -> OutboundSubscription<T> {
        use self::SubscriptionState::*;
        let (mut tx, rx) = mpsc::channel(BUFFER_LEN);
        if let Pending(item) = mem::replace(&mut self.state, NotSubscribed) {
            tx.try_send(item).unwrap();
        }
        self.state = Subscribed(tx);
        OutboundSubscription { inner: rx }
    }

    pub fn is_subscribed(&self) -> bool {
        use self::SubscriptionState::*;

        match self.state {
            Subscribed(_) => true,
            NotSubscribed | Pending(_) => false,
        }
    }

    // Try sending an item to the subscriber.
    // Sending is done as best effort: if the stream buffer is full due to a
    // blockage downstream, a `StreamOverflow` error is returned and
    // the item is dropped.
    // If the subscription is in the pending state with an item already waiting
    // to be sent, the new item replaces the previous pending item.
    pub fn try_send(&mut self, item: T) -> Result<(), PropagateError<T>> {
        match self.state {
            SubscriptionState::NotSubscribed => Err(PropagateError {
                kind: ErrorKind::NotSubscribed,
                item,
            }),
            SubscriptionState::Pending(ref mut pending) => {
                *pending = item;
                Ok(())
            }
            SubscriptionState::Subscribed(ref mut sender) => sender.try_send(item).map_err(|e| {
                if e.is_disconnected() {
                    PropagateError {
                        kind: ErrorKind::SubscriptionClosed,
                        item: e.into_inner(),
                    }
                } else if e.is_full() {
                    PropagateError {
                        kind: ErrorKind::StreamOverflow,
                        item: e.into_inner(),
                    }
                } else {
                    PropagateError {
                        kind: ErrorKind::Unexpected,
                        item: e.into_inner(),
                    }
                }
            }),
        }
    }
}

enum SubscriptionState<T> {
    NotSubscribed,
    Pending(T),
    Subscribed(mpsc::Sender<T>),
}

enum PeerAuth {
    None,
    Authenticated(NodeId),
    ServerNonce([u8; NONCE_LEN]),
}

impl Default for PeerAuth {
    fn default() -> Self {
        PeerAuth::None
    }
}

/// State of the communication streams that a single peer connection polls
/// for outbound data and commands.
///
/// Dropping a `PeerComms` instance results in the client-side connection to
/// be closed if it was established, or all outbound subscription streams of a
/// server-side connection to be closed.
#[derive(Default)]
pub struct PeerComms {
    block_announcements: CommHandle<Header>,
    block_solicitations: CommHandle<BlockIds>,
    chain_pulls: CommHandle<ChainPullRequest>,
    fragments: CommHandle<Fragment>,
    gossip: CommHandle<Gossip>,
    auth: PeerAuth,
}

impl PeerComms {
    pub fn new() -> PeerComms {
        Default::default()
    }

    pub fn has_client_subscriptions(&self) -> bool {
        self.block_announcements.is_client()
            || self.fragments.is_client()
            || self.gossip.is_client()
    }

    pub fn node_id(&self) -> Option<NodeId> {
        match &self.auth {
            PeerAuth::Authenticated(id) => Some(id.clone()),
            _ => None,
        }
    }

    pub fn auth_nonce(&self) -> Option<[u8; NONCE_LEN]> {
        match self.auth {
            PeerAuth::ServerNonce(nonce) => Some(nonce),
            _ => None,
        }
    }

    pub fn generate_auth_nonce(&mut self) -> [u8; NONCE_LEN] {
        let mut nonce = [0u8; NONCE_LEN];
        rand::thread_rng().fill(&mut nonce[..]);
        self.auth = PeerAuth::ServerNonce(nonce);
        nonce
    }

    pub fn set_node_id(&mut self, id: NodeId) {
        self.auth = PeerAuth::Authenticated(id);
    }

    pub fn update(&mut self, newer: PeerComms) {
        // If there would be a need to tell the old connection that
        // it is replaced in any better way than just dropping all its
        // communiction handles, this is the place to do it.
        self.block_announcements.update(newer.block_announcements);
        self.fragments.update(newer.fragments);
        self.gossip.update(newer.gossip);
        self.block_solicitations.update(newer.block_solicitations);
        self.chain_pulls.update(newer.chain_pulls);
        self.auth = newer.auth;
    }

    pub fn clear_pending(&mut self) {
        self.block_announcements.clear_pending();
        self.fragments.clear_pending();
        self.gossip.clear_pending();
        self.block_solicitations.clear_pending();
        self.chain_pulls.clear_pending();
    }

    pub fn set_pending_block_announcement(&mut self, header: Header) {
        self.block_announcements = CommHandle::client_pending(header);
    }

    pub fn set_pending_fragment(&mut self, fragment: Fragment) {
        self.fragments = CommHandle::client_pending(fragment);
    }

    pub fn set_pending_gossip(&mut self, gossip: Gossip) {
        self.gossip = CommHandle::client_pending(gossip);
    }

    pub fn try_send_block_announcement(
        &mut self,
        header: Header,
    ) -> Result<(), PropagateError<Header>> {
        self.block_announcements.try_send(header)
    }

    pub fn try_send_fragment(
        &mut self,
        fragment: Fragment,
    ) -> Result<(), PropagateError<Fragment>> {
        self.fragments.try_send(fragment)
    }

    pub fn try_send_gossip(&mut self, gossip: Gossip) -> Result<(), PropagateError<Gossip>> {
        self.gossip.try_send(gossip)
    }

    pub fn subscribe_to_block_announcements(&mut self) -> OutboundSubscription<Header> {
        self.block_announcements.subscribe()
    }

    pub fn subscribe_to_block_solicitations(&mut self) -> OutboundSubscription<BlockIds> {
        self.block_solicitations.subscribe()
    }

    pub fn subscribe_to_chain_pulls(&mut self) -> OutboundSubscription<ChainPullRequest> {
        self.chain_pulls.subscribe()
    }

    pub fn subscribe_to_block_events(&mut self) -> BlockEventSubscription {
        let announce_events: BlockEventAnnounceStream = self
            .block_announcements
            .subscribe()
            .map(BlockEvent::Announce);
        let solicit_events: BlockEventSolicitStream = self
            .block_solicitations
            .subscribe()
            .map(BlockEvent::Solicit);
        let missing_events: BlockEventMissingStream =
            self.chain_pulls.subscribe().map(BlockEvent::Missing);
        stream::select(
            announce_events,
            stream::select(solicit_events, missing_events),
        )
    }

    pub fn subscribe_to_fragments(&mut self) -> FragmentSubscription {
        self.fragments.subscribe()
    }

    pub fn subscribe_to_gossip(&mut self) -> GossipSubscription {
        self.gossip.subscribe()
    }

    pub fn block_announcements_subscribed(&self) -> bool {
        self.block_announcements.is_subscribed()
    }

    pub fn fragments_subscribed(&self) -> bool {
        self.fragments.is_subscribed()
    }

    pub fn gossip_subscribed(&self) -> bool {
        self.gossip.is_subscribed()
    }
}

/// Options for Peers::add_connecting
#[derive(Default)]
pub struct ConnectOptions {
    /// Block announcement to send once the subscription is established
    pub pending_block_announcement: Option<Header>,
    /// Fragment to send once the subscription is established
    pub pending_fragment: Option<Fragment>,
    /// Gossip to send once the subscription is established
    pub pending_gossip: Option<Gossip>,
    /// The to number of client connections that need to be removed
    /// prior to connecting.
    pub evict_clients: usize,
}

#[derive(Clone, Debug)]
pub struct PeerStats {
    created: SystemTime,
    last_block_received: Option<SystemTime>,
    last_fragment_received: Option<SystemTime>,
    last_gossip_received: Option<SystemTime>,
}

impl Default for PeerStats {
    fn default() -> Self {
        PeerStats {
            created: SystemTime::now(),
            last_block_received: None,
            last_fragment_received: None,
            last_gossip_received: None,
        }
    }
}

impl PeerStats {
    pub fn last_block_received(&self) -> Option<SystemTime> {
        self.last_block_received
    }

    pub fn last_fragment_received(&self) -> Option<SystemTime> {
        self.last_fragment_received
    }

    pub fn last_gossip_received(&self) -> Option<SystemTime> {
        self.last_gossip_received
    }

    fn update_last_block_received(&mut self, timestamp: SystemTime) {
        update_last_timestamp(&mut self.last_block_received, timestamp)
    }

    fn update_last_fragment_received(&mut self, timestamp: SystemTime) {
        update_last_timestamp(&mut self.last_fragment_received, timestamp)
    }

    fn update_last_gossip_received(&mut self, timestamp: SystemTime) {
        update_last_timestamp(&mut self.last_gossip_received, timestamp)
    }

    pub fn connection_established(&self) -> SystemTime {
        self.created
    }

    pub fn last_activity(&self) -> SystemTime {
        use std::cmp::max;

        let last_block_received = self.last_block_received.unwrap_or(self.created);
        let last_fragment_received = self.last_fragment_received.unwrap_or(self.created);
        let last_gossip_received = self.last_gossip_received.unwrap_or(self.created);

        max(
            last_block_received,
            max(last_fragment_received, last_gossip_received),
        )
    }
}

fn update_last_timestamp(field: &mut Option<SystemTime>, timestamp: SystemTime) {
    match *field {
        None => {
            *field = Some(timestamp);
        }
        Some(last) if last < timestamp => {
            *field = Some(timestamp);
        }
        _ => {}
    }
}

#[derive(Debug)]
pub struct PeerInfo {
    pub addr: Option<SocketAddr>,
    pub stats: PeerStats,
}

/// The collection of currently connected peer nodes.
///
/// This object uses internal locking and is shared between
/// all network connection tasks.
pub struct Peers {
    mutex: Mutex<PeerMap>,
    span: Span,
}

impl Peers {
    pub fn new(capacity: usize, span: Span) -> Self {
        Peers {
            mutex: Mutex::new(PeerMap::new(capacity)),
            span,
        }
    }

    fn inner(&self) -> MutexLockFuture<PeerMap> {
        self.mutex.lock()
    }

    pub async fn clear(&self) {
        let mut map = self.inner().await;
        map.clear()
    }

    pub async fn add_connecting(
        &self,
        peer: Address,
        handle: ConnectHandle,
        options: ConnectOptions,
    ) {
        async move {
            if options.evict_clients != 0 {
                tracing::debug!("will evict {} clients", options.evict_clients);
            }
            let mut map = self.inner().await;
            map.evict_clients(options.evict_clients);
            let comms = map.add_connecting(peer, handle);
            if let Some(header) = options.pending_block_announcement {
                comms.set_pending_block_announcement(header);
            }
            if let Some(fragment) = options.pending_fragment {
                comms.set_pending_fragment(fragment);
            }
            if let Some(gossip) = options.pending_gossip {
                comms.set_pending_gossip(gossip);
            }
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn update_entry(&self, peer: Address) {
        if let Some(ref mut peer) = self.inner().await.entry(peer) {
            peer.update_comm_status();
        }
    }

    pub async fn remove_peer(&self, peer: Address) -> Option<PeerComms> {
        async move {
            let mut map = self.inner().await;
            map.remove_peer(peer)
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn generate_auth_nonce(&self, peer: Address) -> [u8; NONCE_LEN] {
        async move {
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.generate_auth_nonce()
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn get_auth_nonce(&self, peer: Address) -> Option<[u8; NONCE_LEN]> {
        async move {
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.auth_nonce()
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn set_node_id(&self, peer: Address, id: NodeId) {
        async move {
            tracing::debug!(
                peer = %peer,
                node_id = ?id,
                "authenticated client peer node"
            );
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.set_node_id(id);
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn subscribe_to_block_events(&self, peer: Address) -> BlockEventSubscription {
        async move {
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.subscribe_to_block_events()
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn subscribe_to_fragments(&self, peer: Address) -> FragmentSubscription {
        async move {
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.subscribe_to_fragments()
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn subscribe_to_gossip(&self, peer: Address) -> GossipSubscription {
        async move {
            let mut map = self.inner().await;
            let comms = map.server_comms(peer);
            comms.subscribe_to_gossip()
        }
        .instrument(self.span.clone())
        .await
    }

    async fn propagate_with<T, F>(&self, nodes: Vec<Address>, f: F) -> Result<(), Vec<Address>>
    where
        for<'a> F: Fn(CommStatus<'a>) -> Result<(), PropagateError<T>>,
    {
        let mut map = self.inner().await;
        let mut reached_node_ids = HashSet::new();
        let unreached_nodes = nodes
            .into_iter()
            .filter(move |node| {
                if let Some(mut entry) = map.entry(*node) {
                    let comm_status = entry.update_comm_status();
                    let node_id = comm_status.node_id();

                    // Avoid propagating more than once to the same node
                    if let Some(id) = &node_id {
                        if reached_node_ids.contains(id) {
                            tracing::debug!(
                                peer = %node,
                                node_id = ?id,
                                "node ID has been reached via another peer connection, omitting"
                            );
                            return false;
                        }
                    }

                    match f(comm_status) {
                        Ok(()) => {
                            if let Some(id) = node_id {
                                reached_node_ids.insert(id);
                            }
                            false
                        }
                        Err(e) => {
                            tracing::debug!(
                                peer = %node,
                                reason = %e.kind(),
                                "propagation to peer failed, unsubscribing peer"
                            );
                            entry.remove();
                            true
                        }
                    }
                } else {
                    true
                }
            })
            .collect::<Vec<_>>();
        if unreached_nodes.is_empty() {
            Ok(())
        } else {
            Err(unreached_nodes)
        }
    }

    pub async fn propagate_block(
        &self,
        nodes: Vec<Address>,
        header: Header,
    ) -> Result<(), Vec<Address>> {
        async move {
            tracing::debug!("propagating block to {:?}", nodes);
            self.propagate_with(nodes, move |status| match status {
                CommStatus::Established(comms) => comms.try_send_block_announcement(header.clone()),
                CommStatus::Connecting(comms) => {
                    comms.set_pending_block_announcement(header.clone());
                    Ok(())
                }
            })
            .await
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn propagate_fragment(
        &self,
        nodes: Vec<Address>,
        fragment: Fragment,
    ) -> Result<(), Vec<Address>> {
        async move {
            tracing::debug!("propagating fragment to {:?}", nodes);
            self.propagate_with(nodes, move |status| match status {
                CommStatus::Established(comms) => comms.try_send_fragment(fragment.clone()),
                CommStatus::Connecting(comms) => {
                    comms.set_pending_fragment(fragment.clone());
                    Ok(())
                }
            })
            .await
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn propagate_gossip_to(&self, target: Address, gossip: Gossip) -> Result<(), Gossip> {
        async move {
            tracing::debug!(
                peer = %target,
                "sending gossip"
            );
            let mut map = self.inner().await;
            if let Some(mut entry) = map.entry(target) {
                let res = match entry.update_comm_status() {
                    CommStatus::Established(comms) => comms.try_send_gossip(gossip),
                    CommStatus::Connecting(comms) => {
                        comms.set_pending_gossip(gossip);
                        Ok(())
                    }
                };
                res.map_err(|e| {
                    tracing::debug!(
                        peer = %entry.address(),
                        reason = %e.kind(),
                        "gossip propagation to peer failed, unsubscribing peer"
                    );
                    entry.remove();
                    e.into_item()
                })
            } else {
                Err(gossip)
            }
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn refresh_peer_on_block(&self, peer: Address) -> bool {
        let timestamp = SystemTime::now();
        let mut map = self.inner().await;
        match map.refresh_peer(&peer) {
            Some(stats) => {
                stats.update_last_block_received(timestamp);
                true
            }
            None => false,
        }
    }

    pub async fn refresh_peer_on_fragment(&self, peer: Address) -> bool {
        let timestamp = SystemTime::now();
        let mut map = self.inner().await;
        match map.refresh_peer(&peer) {
            Some(stats) => {
                stats.update_last_fragment_received(timestamp);
                true
            }
            None => false,
        }
    }

    pub async fn refresh_peer_on_gossip(&self, peer: Address) -> bool {
        let timestamp = SystemTime::now();
        let mut map = self.inner().await;
        match map.refresh_peer(&peer) {
            Some(stats) => {
                stats.update_last_gossip_received(timestamp);
                true
            }
            None => false,
        }
    }

    pub async fn fetch_blocks(&self, hashes: BlockIds) {
        async move {
            let mut map = self.inner().await;
            if let Some((node_id, comms)) = map.next_peer_for_block_fetch() {
                tracing::debug!("fetching blocks from {}", node_id);
                comms
                    .block_solicitations
                    .try_send(hashes)
                    .unwrap_or_else(|e| {
                        tracing::debug!("block fetch from {} failed: {:?}", node_id, e);
                        tracing::debug!("unsubscribing peer {}", node_id);
                        map.remove_peer(node_id);
                    });
            } else {
                tracing::warn!("no peers to fetch blocks from");
            }
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn solicit_blocks(&self, peer: Address, hashes: BlockIds) {
        async move {
            let mut map = self.inner().await;
            match map.peer_comms(&peer) {
                Some(comms) => {
                    tracing::debug!(
                        peer = %peer,
                        hashes = ?hashes,
                        "sending block solicitation"
                    );
                    comms
                        .block_solicitations
                        .try_send(hashes)
                        .unwrap_or_else(|e| {
                            tracing::debug!(
                                peer = %peer,
                                error = ?e,
                                "sending block solicitation failed, unsubscribing"
                            );
                            map.remove_peer(peer);
                        });
                }
                None => {
                    // TODO: connect and request on demand, or select another peer?
                    tracing::info!(
                        peer = %peer,
                        "peer not available to solicit blocks from"
                    );
                }
            }
        }
        .instrument(self.span.clone())
        .await
    }

    pub async fn pull_headers(&self, peer: Address, from: BlockIds, to: BlockId) {
        async move {
            let mut map = self.inner().await;
            match map.peer_comms(&peer) {
                Some(comms) => {
                    tracing::debug!(
                    peer = %peer,
                    from = %format!("[{}]", from.iter().map(hex::encode).collect::<Vec<_>>().join(", ")),
                    to = %hex::encode(to),
                    "pulling headers"
                );
                    comms
                        .chain_pulls
                        .try_send(ChainPullRequest { from, to })
                        .unwrap_or_else(|e| {
                            tracing::debug!(
                            peer = %peer,
                            error = ?e,
                            "sending header pull solicitation failed, unsubscribing"
                        );
                            map.remove_peer(peer);
                        });
                }
                None => {
                    // TODO: connect and request on demand, or select another peer?
                    tracing::info!(
                    peer = %peer,
                    "peer not available to pull headers from"
                );
                }
            }
        }.instrument(self.span.clone()).await
    }

    pub async fn infos(&self) -> Vec<PeerInfo> {
        let map = self.inner().await;
        map.infos()
    }
}
