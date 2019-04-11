use super::p2p_topology as p2p;
use crate::blockcfg::{Header, Message};

use network_core::{
    error::Error,
    gossip::{Gossip, Node},
};

use futures::prelude::*;
use futures::sync::mpsc;

use std::{
    collections::{hash_map, HashMap},
    sync::Mutex,
};

// Buffer size determines the number of stream items pending processing that
// can be buffered before back pressure is applied to the inbound half of
// a gRPC subscription stream.
const BUFFER_LEN: usize = 8;

#[derive(Debug)]
pub enum PropagateError {
    NotSubscribed,
    SubscriptionClosed,
    StreamOverflow,
    Unexpected,
}

/// Stream used to send propagated items to the outbound half of
/// a subscription stream.
pub struct Subscription<T> {
    inner: mpsc::Receiver<T>,
}

impl<T> Stream for Subscription<T> {
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.inner.poll().unwrap())
    }
}

/// Handle used by the per-peer connection tasks to produce an outbound
/// subscription stream towards the peer.
pub struct PropagationHandle<T> {
    state: SubscriptionState<T>,
}

impl<T> Default for PropagationHandle<T> {
    fn default() -> Self {
        PropagationHandle {
            state: SubscriptionState::NotSubscribed,
        }
    }
}

impl<T> PropagationHandle<T> {
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
    // blockage downstream, an `Err(PropagateError::StreamOverflow)` is
    // returned and the item is dropped.
    pub fn try_send(&mut self, item: T) -> Result<(), PropagateError> {
        match self.state {
            SubscriptionState::NotSubscribed => Err(PropagateError::NotSubscribed),
            SubscriptionState::Subscribed(ref mut sender) => sender.try_send(item).map_err(|e| {
                if e.is_disconnected() {
                    PropagateError::SubscriptionClosed
                } else if e.is_full() {
                    PropagateError::StreamOverflow
                } else {
                    PropagateError::Unexpected
                }
            }),
        }
    }
}

enum SubscriptionState<T> {
    NotSubscribed,
    Subscribed(mpsc::Sender<T>),
}

/// Propagation subscription handles for all stream types that a peer can
/// be subscribed to.
#[derive(Default)]
pub struct PeerHandles {
    pub blocks: PropagationHandle<Header>,
    pub messages: PropagationHandle<Message>,
    pub gossip: PropagationHandle<Gossip<p2p::Node>>,
}

impl PeerHandles {
    pub fn new() -> PeerHandles {
        PeerHandles {
            ..Default::default()
        }
    }

    pub fn try_send_block(&mut self, header: Header) -> Result<(), PropagateError> {
        self.blocks.try_send(header)
    }

    pub fn try_send_message(&mut self, message: Message) -> Result<(), PropagateError> {
        self.messages.try_send(message)
    }

    pub fn try_send_gossip(&mut self, gossip: Gossip<p2p::Node>) -> Result<(), PropagateError> {
        self.gossip.try_send(gossip)
    }
}

/// The map of peer nodes currently subscribed to chain or network updates.
///
/// This map object uses internal locking and is shared between
/// all network connection tasks.
pub struct PropagationMap {
    mutex: Mutex<HashMap<p2p::NodeId, PeerHandles>>,
}

fn ensure_propagation_peer<'a>(
    map: &'a mut HashMap<p2p::NodeId, PeerHandles>,
    id: p2p::NodeId,
) -> &'a mut PeerHandles {
    map.entry(id).or_insert(PeerHandles::new())
}

impl PropagationMap {
    pub fn new() -> Self {
        PropagationMap {
            mutex: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert_peer(&self, id: p2p::NodeId, handles: PeerHandles) {
        let mut map = self.mutex.lock().unwrap();
        map.insert(id, handles);
    }

    pub fn subscribe_to_blocks(&self, id: p2p::NodeId) -> Subscription<Header> {
        let mut map = self.mutex.lock().unwrap();
        let handles = ensure_propagation_peer(&mut map, id);
        handles.blocks.subscribe()
    }

    pub fn subscribe_to_messages(&self, id: p2p::NodeId) -> Subscription<Message> {
        let mut map = self.mutex.lock().unwrap();
        let handles = ensure_propagation_peer(&mut map, id);
        handles.messages.subscribe()
    }

    pub fn subscribe_to_gossip(&self, id: p2p::NodeId) -> Subscription<Gossip<p2p::Node>> {
        let mut map = self.mutex.lock().unwrap();
        let handles = ensure_propagation_peer(&mut map, id);
        handles.gossip.subscribe()
    }

    fn propagate_with<F>(&self, nodes: Vec<p2p::Node>, f: F) -> Result<(), Vec<p2p::Node>>
    where
        F: Fn(&mut PeerHandles) -> Result<(), PropagateError>,
    {
        let mut map = self.mutex.lock().unwrap();
        let unreached_nodes = nodes
            .into_iter()
            .filter(|node| {
                let id = node.id();
                if let hash_map::Entry::Occupied(mut entry) = map.entry(id) {
                    match f(entry.get_mut()) {
                        Ok(()) => false,
                        Err(e) => {
                            info!("propagation to peer {} failed: {:?}", id, e);
                            debug!("unsubscribing peer {}", id);
                            entry.remove_entry();
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
        nodes: Vec<p2p::Node>,
        header: Header,
    ) -> Result<(), Vec<p2p::Node>> {
        self.propagate_with(nodes, |handles| handles.try_send_block(header.clone()))
    }

    pub fn propagate_message(
        &self,
        nodes: Vec<p2p::Node>,
        message: Message,
    ) -> Result<(), Vec<p2p::Node>> {
        self.propagate_with(nodes, |handles| handles.try_send_message(message.clone()))
    }

    pub fn propagate_gossip(
        &self,
        nodes: Vec<p2p::Node>,
        gossip: Gossip<p2p::Node>,
    ) -> Result<(), Vec<p2p::Node>> {
        self.propagate_with(nodes, |handles| handles.try_send_gossip(gossip.clone()))
    }
}
