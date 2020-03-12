mod peer_map;

use peer_map::{CommStatus, PeerMap};

use crate::blockcfg::{Block, Fragment, Header, HeaderHash};
use crate::network::{
    client::ConnectHandle,
    p2p::{Gossip as NodeData, Id, Node as NodeRef},
};
use chain_network::data::block::{BlockEvent, ChainPullRequest};
use chain_network::data::gossip::{Gossip, Node};
use chain_network::error as net_error;
use futures03::channel::mpsc;
use futures03::lock::{Mutex, MutexLockFuture};
use futures03::prelude::*;
use futures03::stream;
use slog::Logger;

use std::fmt;
use std::mem;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::time::SystemTime;

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
    type Item = Result<T, net_error::Error>;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Ok(self.inner.poll_next(cx))
    }
}

type BlockEventAnnounceStream =
    stream::Map<OutboundSubscription<Header>, fn(Header) -> BlockEvent<Block>>;

type BlockEventSolicitStream =
    stream::Map<OutboundSubscription<Vec<HeaderHash>>, fn(Vec<HeaderHash>) -> BlockEvent<Block>>;

type BlockEventMissingStream = stream::Map<
    OutboundSubscription<ChainPullRequest<HeaderHash>>,
    fn(ChainPullRequest<HeaderHash>) -> BlockEvent<Block>,
>;

pub type BlockEventSubscription = stream::Select<
    stream::Select<BlockEventAnnounceStream, BlockEventSolicitStream>,
    BlockEventMissingStream,
>;

pub type FragmentSubscription = OutboundSubscription<Fragment>;

pub type GossipSubscription = OutboundSubscription<Gossip<NodeData>>;

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
        match self.direction {
            SubscriptionDirection::Client => true,
            _ => false,
        }
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
        match mem::replace(&mut self.state, newer.state) {
            SubscriptionState::Pending(item) => {
                // If there is an error sending the pending item,
                // it is silently dropped. Logging infrastructure to debug
                // this would be nice.
                let _ = self.try_send(item);
            }
            _ => {}
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

/// State of the communication streams that a single peer connection polls
/// for outbound data and commands.
///
/// Dropping a `PeerComms` instance results in the client-side connection to
/// be closed if it was established, or all outbound subscription streams of a
/// server-side connection to be closed.
#[derive(Default)]
pub struct PeerComms {
    block_announcements: CommHandle<Header>,
    block_solicitations: CommHandle<Vec<HeaderHash>>,
    chain_pulls: CommHandle<ChainPullRequest>,
    fragments: CommHandle<Fragment>,
    gossip: CommHandle<Gossip>,
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

    pub fn update(&mut self, newer: PeerComms) {
        // If there would be a need to tell the old connection that
        // it is replaced in any better way than just dropping all its
        // communiction handles, this is the place to do it.
        self.block_announcements.update(newer.block_announcements);
        self.fragments.update(newer.fragments);
        self.gossip.update(newer.gossip);
        self.block_solicitations.update(newer.block_solicitations);
        self.chain_pulls.update(newer.chain_pulls);
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

    pub fn set_pending_gossip(&mut self, gossip: Gossip<NodeData>) {
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

    pub fn try_send_gossip(
        &mut self,
        gossip: Gossip<NodeData>,
    ) -> Result<(), PropagateError<Gossip<NodeData>>> {
        self.gossip.try_send(gossip)
    }

    pub fn subscribe_to_block_announcements(&mut self) -> OutboundSubscription<Header> {
        self.block_announcements.subscribe()
    }

    pub fn subscribe_to_block_solicitations(&mut self) -> OutboundSubscription<Vec<HeaderHash>> {
        self.block_solicitations.subscribe()
    }

    pub fn subscribe_to_chain_pulls(
        &mut self,
    ) -> OutboundSubscription<ChainPullRequest<HeaderHash>> {
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
        announce_events
            .select(solicit_events)
            .select(missing_events)
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
        self.last_block_received.clone()
    }

    pub fn last_fragment_received(&self) -> Option<SystemTime> {
        self.last_fragment_received.clone()
    }

    pub fn last_gossip_received(&self) -> Option<SystemTime> {
        self.last_gossip_received.clone()
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
    pub id: Id,
    pub addr: Option<SocketAddr>,
    pub stats: PeerStats,
}

/// The collection of currently connected peer nodes.
///
/// This object uses internal locking and is shared between
/// all network connection tasks.
#[derive(Clone)]
pub struct Peers {
    mutex: Mutex<PeerMap>,
    logger: Logger,
}

impl Peers {
    pub fn new(capacity: usize, logger: Logger) -> Self {
        Peers {
            mutex: Mutex::new(PeerMap::new(capacity)),
            logger,
        }
    }

    fn inner(&self) -> MutexLockFuture<PeerMap> {
        self.mutex.lock()
    }

    pub fn clear<E>(&self) -> impl Future<Item = (), Error = E> {
        self.inner().map(|mut map| map.clear())
    }

    pub fn insert_peer<E>(
        &self,
        id: Id,
        comms: PeerComms,
        addr: SocketAddr,
    ) -> impl Future<Item = (), Error = E> {
        self.inner()
            .map(move |mut map| map.insert_peer(id, comms, addr))
    }

    pub fn add_connecting<E>(
        &self,
        id: Id,
        handle: ConnectHandle,
        options: ConnectOptions,
    ) -> impl Future<Item = (), Error = E> {
        if options.evict_clients != 0 {
            debug!(self.logger, "will evict {} clients", options.evict_clients);
        }
        self.inner().map(move |mut map| {
            map.evict_clients(options.evict_clients);
            let comms = map.add_connecting(id, handle);
            if let Some(header) = options.pending_block_announcement {
                comms.set_pending_block_announcement(header);
            }
            if let Some(fragment) = options.pending_fragment {
                comms.set_pending_fragment(fragment);
            }
            if let Some(gossip) = options.pending_gossip {
                comms.set_pending_gossip(gossip);
            }
        })
    }

    pub fn remove_peer<E>(&self, id: Id) -> impl Future<Item = Option<PeerComms>, Error = E> {
        self.inner().map(move |mut map| map.remove_peer(id))
    }

    pub fn lock_server_comms(&self, id: Id) -> LockServerComms {
        LockServerComms {
            lock: self.mutex.clone(),
            peer: id,
        }
    }

    fn propagate_with<T, F>(
        &self,
        nodes: Vec<NodeRef>,
        f: F,
    ) -> impl Future<Item = (), Error = Vec<NodeRef>>
    where
        for<'a> F: Fn(CommStatus<'a>) -> Result<(), PropagateError<T>>,
    {
        let logger = self.logger.clone();
        self.inner().and_then(move |mut map| {
            let unreached_nodes = nodes
                .into_iter()
                .filter(|node| {
                    let id = node.id();
                    if let Some(mut entry) = map.entry(id) {
                        match f(entry.update_comm_status()) {
                            Ok(()) => false,
                            Err(e) => {
                                debug!(
                                    logger,
                                    "propagation to peer failed, unsubscribing peer";
                                    "node_id" => %id,
                                    "reason" => %e.kind()
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
        })
    }

    pub fn propagate_block(
        &self,
        nodes: Vec<NodeRef>,
        header: Header,
    ) -> impl Future<Item = (), Error = Vec<NodeRef>> {
        debug!(
            self.logger,
            "propagating block";
            "hash" => %header.hash(),
        );
        self.propagate_with(nodes, move |status| match status {
            CommStatus::Established(comms) => comms.try_send_block_announcement(header.clone()),
            CommStatus::Connecting(comms) => {
                comms.set_pending_block_announcement(header.clone());
                Ok(())
            }
        })
    }

    pub fn propagate_fragment(
        &self,
        nodes: Vec<NodeRef>,
        fragment: Fragment,
    ) -> impl Future<Item = (), Error = Vec<NodeRef>> {
        debug!(
            self.logger,
            "propagating fragment";
        );
        self.propagate_with(nodes, move |status| match status {
            CommStatus::Established(comms) => comms.try_send_fragment(fragment.clone()),
            CommStatus::Connecting(comms) => {
                comms.set_pending_fragment(fragment.clone());
                Ok(())
            }
        })
    }

    pub fn propagate_gossip_to(
        &self,
        target: Id,
        gossip: Gossip<NodeData>,
    ) -> impl Future<Item = (), Error = Gossip<NodeData>> {
        debug!(
            self.logger,
            "sending gossip";
            "node_id" => %target,
        );
        let logger = self.logger.clone();
        self.inner().and_then(move |mut map| {
            if let Some(mut entry) = map.entry(target) {
                let res = match entry.update_comm_status() {
                    CommStatus::Established(comms) => comms.try_send_gossip(gossip),
                    CommStatus::Connecting(comms) => {
                        comms.set_pending_gossip(gossip);
                        Ok(())
                    }
                };
                res.map_err(|e| {
                    debug!(
                        logger,
                        "gossip propagation to peer failed, unsubscribing peer";
                        "node_id" => %target,
                        "reason" => %e.kind(),
                    );
                    entry.remove();
                    e.into_item()
                })
            } else {
                Err(gossip)
            }
        })
    }

    pub fn refresh_peer_on_block<E>(&self, node_id: Id) -> impl Future<Item = bool, Error = E> {
        let timestamp = SystemTime::now();
        self.inner()
            .map(move |mut map| match map.refresh_peer(&node_id) {
                Some(stats) => {
                    stats.update_last_block_received(timestamp);
                    true
                }
                None => false,
            })
    }

    pub fn refresh_peer_on_fragment<E>(&self, node_id: Id) -> impl Future<Item = bool, Error = E> {
        let timestamp = SystemTime::now();
        self.inner()
            .map(move |mut map| match map.refresh_peer(&node_id) {
                Some(stats) => {
                    stats.update_last_fragment_received(timestamp);
                    true
                }
                None => false,
            })
    }

    pub fn refresh_peer_on_gossip<E>(&self, node_id: Id) -> impl Future<Item = bool, Error = E> {
        let timestamp = SystemTime::now();
        self.inner()
            .map(move |mut map| match map.refresh_peer(&node_id) {
                Some(stats) => {
                    stats.update_last_gossip_received(timestamp);
                    true
                }
                None => false,
            })
    }

    pub fn fetch_blocks<E>(&self, hashes: Vec<HeaderHash>) -> impl Future<Item = (), Error = E> {
        let logger = self.logger.clone();
        self.inner().map(move |mut map| {
            if let Some((node_id, comms)) = map.next_peer_for_block_fetch() {
                debug!(logger, "fetching blocks from {}", node_id);
                comms
                    .block_solicitations
                    .try_send(hashes)
                    .unwrap_or_else(|e| {
                        debug!(logger, "block fetch from {} failed: {:?}", node_id, e);
                        debug!(logger, "unsubscribing peer {}", node_id);
                        map.remove_peer(node_id);
                    });
            } else {
                warn!(logger, "no peers to fetch blocks from");
            }
        })
    }

    pub fn solicit_blocks<E>(
        &self,
        node_id: Id,
        hashes: Vec<HeaderHash>,
    ) -> impl Future<Item = (), Error = E> {
        let logger = self.logger.clone();
        self.inner().map(move |mut map| {
            match map.peer_comms(&node_id) {
                Some(comms) => {
                    debug!(logger, "sending block solicitation to {}", node_id;
                       "hashes" => ?hashes);
                    comms
                        .block_solicitations
                        .try_send(hashes)
                        .unwrap_or_else(|e| {
                            debug!(
                                logger,
                                "block solicitation from {} failed: {:?}", node_id, e
                            );
                            debug!(logger, "unsubscribing peer {}", node_id);
                            map.remove_peer(node_id);
                        });
                }
                None => {
                    // TODO: connect and request on demand, or select another peer?
                    info!(
                        logger,
                        "peer {} not available to solicit blocks from", node_id
                    );
                }
            }
        })
    }

    pub fn pull_headers<E>(
        &self,
        node_id: Id,
        from: Vec<HeaderHash>,
        to: HeaderHash,
    ) -> impl Future<Item = (), Error = E> {
        let logger = self.logger.clone();
        self.inner().map(move |mut map| {
        match map.peer_comms(&node_id) {
            Some(comms) => {
                debug!(logger, "pulling headers";
                       "node_id" => %node_id,
                       "from" => format!("[{}]", from.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(", ")),
                       "to" => %to);
                comms
                    .chain_pulls
                    .try_send(ChainPullRequest { from, to })
                    .unwrap_or_else(|e| {
                        debug!(
                            logger,
                            "sending header pull solicitation to {} failed: {:?}", node_id, e
                        );
                        debug!(logger, "unsubscribing peer {}", node_id);
                        map.remove_peer(node_id);
                    });
            }
            None => {
                // TODO: connect and request on demand, or select another peer?
                info!(
                    logger,
                    "peer {} not available to pull headers from", node_id
                );
            }
        }

        })
    }

    pub fn infos<E>(&self) -> impl Future<Item = Vec<PeerInfo>, Error = E> {
        self.inner().map(|map| map.infos())
    }
}

pub struct LockServerComms {
    lock: Mutex<PeerMap>,
    peer: Id,
}

impl LockServerComms {
    pub fn poll_subscribe_with<F, S>(&mut self, f: F) -> Poll<S>
    where
        F: FnOnce(&mut PeerComms) -> S,
    {
        self.lock
            .poll_lock()
            .map(|mut peer_map| f(peer_map.server_comms(self.peer)))
    }
}
