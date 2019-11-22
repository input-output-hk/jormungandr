mod peer_map;

use crate::blockcfg::{Block, Fragment, Header, HeaderHash};
use crate::network::{
    client::ConnectHandle,
    p2p::{Gossip as NodeData, Id, Node as NodeRef},
};
use futures::prelude::*;
use futures::stream;
use futures::sync::mpsc;
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node};
use network_core::subscription::{BlockEvent, ChainPullRequest};
use slog::Logger;

use std::fmt;
use std::mem;
use std::sync::Mutex;
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
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.inner.poll().unwrap())
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

/// Handle used by the per-peer communication tasks to produce an outbound
/// subscription stream towards the peer.
pub struct CommHandle<T> {
    state: SubscriptionState<T>,
}

impl<T> Default for CommHandle<T> {
    fn default() -> Self {
        CommHandle {
            state: SubscriptionState::NotSubscribed,
        }
    }
}

impl<T> CommHandle<T> {
    /// Creates a handle with an item waiting to be sent,
    /// in expectation for a subscription to be established.
    pub fn pending(item: T) -> Self {
        CommHandle {
            state: SubscriptionState::Pending(item),
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
    chain_pulls: CommHandle<ChainPullRequest<HeaderHash>>,
    fragments: CommHandle<Fragment>,
    gossip: CommHandle<Gossip<NodeData>>,
}

impl PeerComms {
    pub fn new() -> PeerComms {
        Default::default()
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
        self.block_announcements = CommHandle::pending(header);
    }

    pub fn set_pending_fragment(&mut self, fragment: Fragment) {
        self.fragments = CommHandle::pending(fragment);
    }

    pub fn set_pending_gossip(&mut self, gossip: Gossip<NodeData>) {
        self.gossip = CommHandle::pending(gossip);
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

    pub fn subscribe_to_fragments(&mut self) -> OutboundSubscription<Fragment> {
        self.fragments.subscribe()
    }

    pub fn subscribe_to_gossip(&mut self) -> OutboundSubscription<Gossip<NodeData>> {
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

/// The collection of currently connected peer nodes.
///
/// This object uses internal locking and is shared between
/// all network connection tasks.
pub struct Peers {
    mutex: Mutex<peer_map::PeerMap>,
    logger: Logger,
}

impl Peers {
    pub fn new(capacity: usize, logger: Logger) -> Self {
        Peers {
            mutex: Mutex::new(peer_map::PeerMap::new(capacity)),
            logger,
        }
    }

    pub fn insert_peer(&self, id: Id, comms: PeerComms) {
        let mut map = self.mutex.lock().unwrap();
        map.insert_peer(id, comms)
    }

    pub fn connecting_with<F>(&self, id: Id, handle: ConnectHandle, modify_comms: F)
    where
        F: FnOnce(&mut PeerComms),
    {
        let mut map = self.mutex.lock().unwrap();
        let comms = map.add_connecting(id, handle);
        modify_comms(comms);
    }

    pub fn remove_peer(&self, id: Id) -> Option<PeerComms> {
        let mut map = self.mutex.lock().unwrap();
        map.remove_peer(id)
    }

    pub fn serve_block_events(&self, id: Id) -> BlockEventSubscription {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.server_comms(id);
        let announce_events: BlockEventAnnounceStream = handles
            .block_announcements
            .subscribe()
            .map(BlockEvent::Announce);
        let solicit_events: BlockEventSolicitStream = handles
            .block_solicitations
            .subscribe()
            .map(BlockEvent::Solicit);
        let missing_events: BlockEventMissingStream =
            handles.chain_pulls.subscribe().map(BlockEvent::Missing);
        announce_events
            .select(solicit_events)
            .select(missing_events)
    }

    pub fn serve_fragments(&self, id: Id) -> OutboundSubscription<Fragment> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.server_comms(id);
        handles.fragments.subscribe()
    }

    pub fn serve_gossip(&self, id: Id) -> OutboundSubscription<Gossip<NodeData>> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.server_comms(id);
        handles.gossip.subscribe()
    }

    fn propagate_with<T, F>(&self, nodes: Vec<NodeRef>, f: F) -> Result<(), Vec<NodeRef>>
    where
        F: Fn(&mut PeerComms) -> Result<(), PropagateError<T>>,
    {
        let mut map = self.mutex.lock().unwrap();
        let unreached_nodes = nodes
            .into_iter()
            .filter(|node| {
                let id = node.id();
                if let Some(mut entry) = map.entry(id) {
                    match f(entry.updated_comms()) {
                        Ok(()) => false,
                        Err(e) => {
                            debug!(
                                self.logger,
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
    }

    pub fn propagate_block(&self, nodes: Vec<NodeRef>, header: Header) -> Result<(), Vec<NodeRef>> {
        debug!(
            self.logger,
            "propagating block";
            "hash" => %header.hash(),
        );
        self.propagate_with(nodes, |handles| {
            handles.try_send_block_announcement(header.clone())
        })
    }

    pub fn propagate_fragment(
        &self,
        nodes: Vec<NodeRef>,
        fragment: Fragment,
    ) -> Result<(), Vec<NodeRef>> {
        debug!(
            self.logger,
            "propagating fragment";
        );
        self.propagate_with(nodes, |handles| handles.try_send_fragment(fragment.clone()))
    }

    pub fn propagate_gossip_to(
        &self,
        target: Id,
        gossip: Gossip<NodeData>,
    ) -> Result<(), Gossip<NodeData>> {
        debug!(
            self.logger,
            "sending gossip";
            "node_id" => %target,
        );
        let mut map = self.mutex.lock().unwrap();
        if let Some(mut entry) = map.entry(target) {
            let res = {
                let handles = entry.updated_comms();
                handles.try_send_gossip(gossip)
            };
            res.map_err(|e| {
                debug!(
                    self.logger,
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
    }

    pub fn refresh_peer_on_block(&self, node_id: Id) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer(node_id) {
            Some(stats) => {
                stats.last_block_received = Some(SystemTime::now());
                true
            }
            None => false,
        }
    }

    pub fn refresh_peer_on_fragment(&self, node_id: Id) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer(node_id) {
            Some(stats) => {
                stats.last_fragment_received = Some(SystemTime::now());
                true
            }
            None => false,
        }
    }

    pub fn refresh_peer_on_gossip(&self, node_id: Id) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer(node_id) {
            Some(stats) => {
                stats.last_gossip_received = Some(SystemTime::now());
                true
            }
            None => false,
        }
    }

    pub fn fetch_blocks(&self, hashes: Vec<HeaderHash>) {
        let mut map = self.mutex.lock().unwrap();
        if let Some((node_id, comms)) = map.next_peer_for_block_fetch() {
            debug!(self.logger, "fetching blocks from {}", node_id);
            comms
                .block_solicitations
                .try_send(hashes)
                .unwrap_or_else(|e| {
                    debug!(self.logger, "block fetch from {} failed: {:?}", node_id, e);
                    debug!(self.logger, "unsubscribing peer {}", node_id);
                    map.remove_peer(node_id);
                });
        } else {
            warn!(self.logger, "no peers to fetch blocks from");
        }
    }

    pub fn solicit_blocks(&self, node_id: Id, hashes: Vec<HeaderHash>) {
        let mut map = self.mutex.lock().unwrap();
        match map.peer_comms(node_id) {
            Some(comms) => {
                debug!(self.logger, "sending block solicitation to {}", node_id;
                       "hashes" => ?hashes);
                comms
                    .block_solicitations
                    .try_send(hashes)
                    .unwrap_or_else(|e| {
                        debug!(
                            self.logger,
                            "block solicitation from {} failed: {:?}", node_id, e
                        );
                        debug!(self.logger, "unsubscribing peer {}", node_id);
                        map.remove_peer(node_id);
                    });
            }
            None => {
                // TODO: connect and request on demand, or select another peer?
                info!(
                    self.logger,
                    "peer {} not available to solicit blocks from", node_id
                );
            }
        }
    }

    pub fn pull_headers(&self, node_id: Id, from: Vec<HeaderHash>, to: HeaderHash) {
        let mut map = self.mutex.lock().unwrap();
        match map.peer_comms(node_id) {
            Some(comms) => {
                debug!(self.logger, "pulling headers";
                       "node_id" => %node_id,
                       "from" => format!("[{}]", from.iter().map(|h| h.to_string()).collect::<Vec<_>>().join(", ")),
                       "to" => %to);
                comms
                    .chain_pulls
                    .try_send(ChainPullRequest { from, to })
                    .unwrap_or_else(|e| {
                        debug!(
                            self.logger,
                            "sending header pull solicitation to {} failed: {:?}", node_id, e
                        );
                        debug!(self.logger, "unsubscribing peer {}", node_id);
                        map.remove_peer(node_id);
                    });
            }
            None => {
                // TODO: connect and request on demand, or select another peer?
                info!(
                    self.logger,
                    "peer {} not available to pull headers from", node_id
                );
            }
        }
    }

    pub fn stats(&self) -> Vec<(Id, PeerStats)> {
        let map = self.mutex.lock().unwrap();
        map.stats()
    }
}
