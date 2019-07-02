mod peer_map;

use super::topology;
use crate::blockcfg::{Block, Header, HeaderHash, Message};
use futures::prelude::*;
use futures::{stream, sync::mpsc};
use network_core::{
    error as core_error,
    gossip::{Gossip, Node},
    subscription::BlockEvent,
};
use slog::Logger;

use std::sync::Mutex;

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

pub type BlockEventSubscription = stream::Select<BlockEventAnnounceStream, BlockEventSolicitStream>;

/// Commands sent by other tasks to translate to requests sent by
/// the client connection.
pub enum ClientCommand {
    PullHeaders {
        from: Vec<HeaderHash>,
        to: HeaderHash,
    },
}

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
    client_commands: Option<mpsc::Sender<ClientCommand>>,
    block_announcements: CommHandle<Header>,
    block_solicitations: CommHandle<Vec<HeaderHash>>,
    messages: CommHandle<Message>,
    gossip: CommHandle<Gossip<topology::Node>>,
}

impl PeerComms {
    pub fn server() -> PeerComms {
        PeerComms {
            ..Default::default()
        }
    }

    pub fn client(commands: mpsc::Sender<ClientCommand>) -> PeerComms {
        PeerComms {
            client_commands: Some(commands),
            block_announcements: Default::default(),
            block_solicitations: Default::default(),
            messages: Default::default(),
            gossip: Default::default(),
        }
    }

    pub fn try_send_block_announcement(
        &mut self,
        header: Header,
    ) -> Result<(), PropagateError<Header>> {
        self.block_announcements.try_send(header)
    }

    pub fn try_send_message(&mut self, message: Message) -> Result<(), PropagateError<Message>> {
        self.messages.try_send(message)
    }

    pub fn try_send_gossip(
        &mut self,
        gossip: Gossip<topology::Node>,
    ) -> Result<(), PropagateError<Gossip<topology::Node>>> {
        self.gossip.try_send(gossip)
    }

    pub fn subscribe_to_block_announcements(&mut self) -> Subscription<Header> {
        self.block_announcements.subscribe()
    }

    pub fn subscribe_to_block_solicitations(&mut self) -> Subscription<Vec<HeaderHash>> {
        self.block_solicitations.subscribe()
    }

    pub fn subscribe_to_messages(&mut self) -> Subscription<Message> {
        self.messages.subscribe()
    }

    pub fn subscribe_to_gossip(&mut self) -> Subscription<Gossip<topology::Node>> {
        self.gossip.subscribe()
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
    pub fn new(logger: Logger) -> Self {
        Peers {
            mutex: Mutex::new(peer_map::PeerMap::new()),
            logger,
        }
    }

    pub fn insert_peer(&self, id: topology::NodeId, comms: PeerComms) {
        let mut map = self.mutex.lock().unwrap();
        map.insert_peer(id, comms)
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
        announce_events.select(solicit_events)
    }

    pub fn subscribe_to_messages(&self, id: topology::NodeId) -> Subscription<Message> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.ensure_peer_comms(id);
        handles.messages.subscribe()
    }

    pub fn subscribe_to_gossip(
        &self,
        id: topology::NodeId,
    ) -> Subscription<Gossip<topology::Node>> {
        let mut map = self.mutex.lock().unwrap();
        let handles = map.ensure_peer_comms(id);
        handles.gossip.subscribe()
    }

    fn propagate_with<T, F>(
        &self,
        nodes: Vec<topology::Node>,
        f: F,
    ) -> Result<(), Vec<topology::Node>>
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
                            info!(
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
        nodes: Vec<topology::Node>,
        header: Header,
    ) -> Result<(), Vec<topology::Node>> {
        self.propagate_with(nodes, |handles| {
            handles.try_send_block_announcement(header.clone())
        })
    }

    pub fn propagate_message(
        &self,
        nodes: Vec<topology::Node>,
        message: Message,
    ) -> Result<(), Vec<topology::Node>> {
        self.propagate_with(nodes, |handles| handles.try_send_message(message.clone()))
    }

    pub fn propagate_gossip_to(
        &self,
        target: topology::NodeId,
        gossip: Gossip<topology::Node>,
    ) -> Result<(), Gossip<topology::Node>> {
        let mut map = self.mutex.lock().unwrap();
        if let Some(mut entry) = map.entry(target) {
            let res = {
                let handles = entry.comms();
                handles.try_send_gossip(gossip)
            };
            res.map_err(|e| {
                info!(
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

    pub fn fetch_blocks(&self, hashes: Vec<HeaderHash>) {
        let mut map = self.mutex.lock().unwrap();
        if let Some((node_id, comms)) = map.next_peer_for_block_fetch() {
            debug!(self.logger, "fetching blocks from {}", node_id);
            comms
                .block_solicitations
                .try_send(hashes)
                .unwrap_or_else(|e| {
                    warn!(
                        self.logger,
                        "block solicitation from {} failed: {:?}", node_id, e
                    );
                });
        } else {
            warn!(self.logger, "no peers to fetch blocks from");
        }
    }

    pub fn solicit_blocks(&self, node_id: topology::NodeId, hashes: Vec<HeaderHash>) {
        let mut map = self.mutex.lock().unwrap();
        match map.peer_comms(node_id) {
            Some(comms) => comms
                .block_solicitations
                .try_send(hashes)
                .unwrap_or_else(|e| {
                    warn!(
                        self.logger,
                        "block solicitation from {} failed: {:?}", node_id, e
                    );
                }),
            None => {
                // TODO: connect and request on demand, or select another peer?
                warn!(
                    self.logger,
                    "peer {} not available to solicit blocks from", node_id
                );
            }
        }
    }

    pub fn pull_headers(&self, node_id: topology::NodeId, from: Vec<HeaderHash>, to: HeaderHash) {
        let mut map = self.mutex.lock().unwrap();
        match map.peer_comms(node_id) {
            Some(PeerComms {
                client_commands: Some(command_queue),
                ..
            }) => command_queue
                .try_send(ClientCommand::PullHeaders { from, to })
                .unwrap_or_else(|e| {
                    warn!(
                        self.logger,
                        "block solicitation from {} failed: {:?}", node_id, e
                    );
                }),
            Some(_non_client_comms) => {
                // TODO: send a new type of solicitation event to retrieve
                // the chain of headers, or straight UploadBlocks
                warn!(
                    self.logger,
                    "peer {} is connected as a client, can't pull headers from it", node_id
                );
            }
            None => {
                // TODO: connect and request on demand, or select another peer?
                warn!(
                    self.logger,
                    "peer {} not available to pull headers from", node_id
                );
            }
        }
    }
}
