use super::p2p_topology as p2p;
use crate::blockcfg::{Header, Message};

use network_core::{error::Error, gossip::Gossip};

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

    fn propagate_with<F>(&self, ids: &[p2p::NodeId], f: F)
    where
        F: Fn(&mut PeerHandles) -> Result<(), PropagateError>,
    {
        let mut map = self.mutex.lock().unwrap();
        for id in ids {
            if let hash_map::Entry::Occupied(mut entry) = map.entry(id.clone()) {
                f(entry.get_mut()).unwrap_or_else(|e| {
                    info!("propagation stream error: {:?}", e);
                    debug!("unsubscribing peer {}", id);
                    entry.remove_entry();
                });
            } else {
                // TODO: connect asynchronously and deliver the item
                // once subscribed
            }
        }
    }

    pub fn propagate_block(&self, ids: &[p2p::NodeId], header: Header) {
        self.propagate_with(ids, |handles| handles.blocks.try_send(header.clone()))
    }

    pub fn propagate_message(&self, ids: &[p2p::NodeId], message: Message) {
        self.propagate_with(ids, |handles| handles.messages.try_send(message.clone()))
    }

    pub fn propagate_gossip(&self, ids: &[p2p::NodeId], gossip: Gossip<p2p::Node>) {
        self.propagate_with(ids, |handles| handles.gossip.try_send(gossip.clone()))
    }
}
