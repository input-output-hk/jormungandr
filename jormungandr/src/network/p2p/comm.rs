mod peer_map;

use super::topology;
use crate::blockcfg::{Block, Fragment, Header, HeaderHash};
use futures::prelude::*;
use futures::{stream, sync::mpsc};
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node};
use network_core::subscription::{BlockEvent, ChainPullRequest};
use slog::Logger;

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

/// Stream used as the outbound half of a subscription stream.
pub struct Subscription<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> Stream for Subscription<T> {
    type Item = T;
    type Error = core_error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.inner.poll().unwrap())
    }
}

type BlockEventAnnounceStream = stream::Map<Subscription<Header>, fn(Header) -> BlockEvent<Block>>;

type BlockEventSolicitStream =
    stream::Map<Subscription<Vec<HeaderHash>>, fn(Vec<HeaderHash>) -> BlockEvent<Block>>;

type BlockEventMissingStream = stream::Map<
    Subscription<ChainPullRequest<HeaderHash>>,
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
    /// Returns a stream to use as an outbound half of the
    /// subscription stream.
    ///
    /// If this method is called again on the same handle,
    /// the previous subscription is closed and its stream is terminated.
    pub fn subscribe(&mut self) -> Subscription<T> {
        let (tx, rx) = mpsc::channel(BUFFER_LEN);
        self.state = SubscriptionState::Subscribed(tx);
        Subscription { inner: rx }
    }

    // Try sending the item to the subscriber.
    // Sending is done as best effort: if the stream buffer is full due to a
    // blockage downstream, a `StreamOverflow` error is
    // returned and the item is dropped.
    pub fn try_send(&mut self, item: T) -> Result<(), PropagateError<T>> {
        match self.state {
            SubscriptionState::NotSubscribed => Err(PropagateError {
                kind: ErrorKind::NotSubscribed,
                item,
            }),
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
    gossip: CommHandle<Gossip<topology::NodeData>>,
    stats: PeerStats,
}

impl PeerComms {
    pub fn new() -> PeerComms {
        Default::default()
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
        gossip: Gossip<topology::NodeData>,
    ) -> Result<(), PropagateError<Gossip<topology::NodeData>>> {
        self.gossip.try_send(gossip)
    }

    pub fn subscribe_to_block_announcements(&mut self) -> Subscription<Header> {
        self.block_announcements.subscribe()
    }

    pub fn subscribe_to_block_solicitations(&mut self) -> Subscription<Vec<HeaderHash>> {
        self.block_solicitations.subscribe()
    }

    pub fn subscribe_to_chain_pulls(&mut self) -> Subscription<ChainPullRequest<HeaderHash>> {
        self.chain_pulls.subscribe()
    }

    pub fn subscribe_to_fragments(&mut self) -> Subscription<Fragment> {
        self.fragments.subscribe()
    }

    pub fn subscribe_to_gossip(&mut self) -> Subscription<Gossip<topology::NodeData>> {
        self.gossip.subscribe()
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

    pub fn insert_peer(&self, id: topology::NodeId, comms: PeerComms) {
        let mut map = self.mutex.lock().unwrap();
        map.insert_peer(id, comms)
    }

    pub fn remove_peer(&self, id: topology::NodeId) {
        let mut map = self.mutex.lock().unwrap();
        map.remove_peer(id);
    }

    pub fn subscribe_to_block_events(&self, id: topology::NodeId) -> BlockEventSubscription {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.ensure_peer_comms(id);
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

    pub fn subscribe_to_fragments(&self, id: topology::NodeId) -> Subscription<Fragment> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.ensure_peer_comms(id);
        handles.fragments.subscribe()
    }

    pub fn subscribe_to_gossip(
        &self,
        id: topology::NodeId,
    ) -> Subscription<Gossip<topology::NodeData>> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.ensure_peer_comms(id);
        handles.gossip.subscribe()
    }

    fn propagate_with<T, F>(
        &self,
        nodes: Vec<topology::NodeData>,
        f: F,
    ) -> Result<(), Vec<topology::NodeData>>
    where
        F: Fn(&mut PeerComms) -> Result<(), PropagateError<T>>,
    {
        let mut map = self.mutex.lock().unwrap();
        let unreached_nodes = nodes
            .into_iter()
            .filter(|node| {
                let id = node.id();
                if let Some(mut entry) = map.entry(id) {
                    match f(entry.comms()) {
                        Ok(()) => false,
                        Err(e) => {
                            debug!(
                                self.logger,
                                "propagation to peer {} failed: {:?}",
                                id,
                                e.kind()
                            );
                            debug!(self.logger, "unsubscribing peer {}", id);
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

    pub fn propagate_block(
        &self,
        nodes: Vec<topology::NodeData>,
        header: Header,
    ) -> Result<(), Vec<topology::NodeData>> {
        self.propagate_with(nodes, |handles| {
            handles.try_send_block_announcement(header.clone())
        })
    }

    pub fn propagate_fragment(
        &self,
        nodes: Vec<topology::NodeData>,
        fragment: Fragment,
    ) -> Result<(), Vec<topology::NodeData>> {
        self.propagate_with(nodes, |handles| handles.try_send_fragment(fragment.clone()))
    }

    pub fn propagate_gossip_to(
        &self,
        target: topology::NodeId,
        gossip: Gossip<topology::NodeData>,
    ) -> Result<(), Gossip<topology::NodeData>> {
        let mut map = self.mutex.lock().unwrap();
        if let Some(mut entry) = map.entry(target) {
            let res = {
                let handles = entry.comms();
                handles.try_send_gossip(gossip)
            };
            res.map_err(|e| {
                debug!(
                    self.logger,
                    "gossip propagation to peer {} failed: {:?}",
                    target,
                    e.kind()
                );
                debug!(self.logger, "unsubscribing peer {}", target);
                entry.remove();
                e.into_item()
            })
        } else {
            Err(gossip)
        }
    }

    pub fn refresh_peer_on_block(&self, node_id: topology::NodeId) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer_comms(node_id) {
            Some(comms) => {
                comms.stats.last_block_received = Some(SystemTime::now());
                true
            }
            None => false,
        }
    }

    pub fn refresh_peer_on_fragment(&self, node_id: topology::NodeId) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer_comms(node_id) {
            Some(comms) => {
                comms.stats.last_fragment_received = Some(SystemTime::now());
                true
            }
            None => false,
        }
    }

    pub fn refresh_peer_on_gossip(&self, node_id: topology::NodeId) -> bool {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer_comms(node_id) {
            Some(comms) => {
                comms.stats.last_gossip_received = Some(SystemTime::now());
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
                    debug!(
                        self.logger,
                        "block solicitation from {} failed: {:?}", node_id, e
                    );
                    debug!(self.logger, "unsubscribing peer {}", node_id);
                    map.remove_peer(node_id);
                });
        } else {
            warn!(self.logger, "no peers to fetch blocks from");
        }
    }

    pub fn solicit_blocks(&self, node_id: topology::NodeId, hashes: Vec<HeaderHash>) {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer_comms(node_id) {
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

    pub fn pull_headers(&self, node_id: topology::NodeId, from: Vec<HeaderHash>, to: HeaderHash) {
        let mut map = self.mutex.lock().unwrap();
        match map.refresh_peer_comms(node_id) {
            Some(comms) => {
                debug!(self.logger, "pulling headers from {}", node_id;
                       "from" => ?from, "to" => ?to);
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

    pub fn stats(&self) -> Vec<(topology::NodeId, PeerStats)> {
        let mut map = self.mutex.lock().unwrap();
        map.stats()
    }
}
