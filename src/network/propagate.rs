use super::p2p_topology::Node;
use crate::blockcfg::{Header, Message};

use network_core::{error::Error, gossip::Gossip};

use futures::prelude::*;
use futures::sync::mpsc;

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
    pub gossip: PropagationHandle<Gossip<Node>>,
}

impl PeerHandles {
    pub fn new() -> PeerHandles {
        PeerHandles {
            ..Default::default()
        }
    }
}
