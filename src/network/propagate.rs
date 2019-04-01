use super::p2p_topology::{Node, NodeId};
use crate::blockcfg::{Header, Message};

use network_core::{
    error::{Code, Error},
    gossip::Gossip,
};

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};

use std::collections::HashMap;

// Buffer size determines the number of stream items pending processing that
// can be buffered before back pressure is applied to the inbound half of
// a gRPC subscription stream.
const BUFFER_LEN: usize = 8;

#[derive(Debug)]
pub enum PropagateError {
    NotSubscribed,
    SubscriptionClosed,
    StreamOverflow,
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
    sub_oneshot: Option<oneshot::Sender<mpsc::Sender<T>>>,
}

impl<T> PropagationHandle<T> {
    /// Reports to the propagation registry that a subscription stream
    /// has been established and returns the propagation stream to feed
    /// into the subscription.
    ///
    /// # Errors
    ///
    /// This operation fails with error code `FailedPrecondition` on all
    /// attempts to establish a subscription except the first.
    pub fn subscribe(&mut self) -> Result<Subscription<T>, Error> {
        // This does not really have to be a one-shot, could permit
        // resubscriptions by dropping the previous sender on the receiving end.
        match self.sub_oneshot.take() {
            None => Err(Error::new(
                Code::FailedPrecondition,
                "subscription already established",
            )),
            Some(oneshot) => {
                let (sender, receiver) = mpsc::channel(BUFFER_LEN);
                match oneshot.send(sender) {
                    Ok(()) => Ok(Subscription { inner: receiver }),
                    Err(_) => Err(Error::new(Code::Canceled, "subscription canceled")),
                }
            }
        }
    }
}

enum SubscriptionState<T> {
    Pending(oneshot::Receiver<mpsc::Sender<T>>),
    Established(mpsc::Sender<T>),
}

impl<T> SubscriptionState<T> {
    fn try_send(&mut self, item: T) -> Result<(), PropagateError> {
        match self {
            SubscriptionState::Pending(_) => Err(PropagateError::NotSubscribed),
            SubscriptionState::Established(sender) => {
                // Try sending the item as best effort.
                // If the stream buffer is full due to a logjam
                // downstream, drop the item and report failure.
                sender.try_send(item).map_err(|e| {
                    if e.is_disconnected() {
                        PropagateError::SubscriptionClosed
                    } else if e.is_full() {
                        PropagateError::StreamOverflow
                    } else {
                        unreachable!()
                    }
                })
            }
        }
    }
}

fn pending_subscription<T>() -> (PropagationHandle<T>, SubscriptionState<T>) {
    let (sender, receiver) = oneshot::channel();
    let handle = PropagationHandle {
        sub_oneshot: Some(sender),
    };
    let state = SubscriptionState::Pending(receiver);
    (handle, state)
}

/// Propagation subscription handles for all stream types that a peer can
/// be subscribed to.
pub struct PeerHandles {
    pub blocks: PropagationHandle<Header>,
    pub messages: PropagationHandle<Message>,
    pub gossip: PropagationHandle<Gossip<Node>>,
}

struct PeerStates {
    pub blocks: SubscriptionState<Header>,
    pub messages: SubscriptionState<Message>,
    pub gossip: SubscriptionState<Gossip<Node>>,
}

/// Maintains the state of active peer subscriptions.
pub struct Propagator {
    subscriptions: HashMap<NodeId, PeerStates>,
}

impl Propagator {
    /// Registers a remote peer ID for a client or server connection
    /// and returns the subscription handles to pass to the network task.
    ///
    /// If the peer was previously registered and had active subscriptions,
    /// the subscription streams are closed.
    pub fn add_peer(&mut self, id: NodeId) -> PeerHandles {
        let (blocks_handle, blocks_state) = pending_subscription();
        let (messages_handle, messages_state) = pending_subscription();
        let (gossip_handle, gossip_state) = pending_subscription();
        let states = PeerStates {
            blocks: blocks_state,
            messages: messages_state,
            gossip: gossip_state,
        };
        self.subscriptions.insert(id, states);
        PeerHandles {
            blocks: blocks_handle,
            messages: messages_handle,
            gossip: gossip_handle,
        }
    }

    pub fn send_block(&mut self, id: NodeId, header: Header) -> Result<(), PropagateError> {
        use std::collections::hash_map::Entry::*;
        match self.subscriptions.entry(id) {
            Vacant(_) => {
                return Err(PropagateError::NotSubscribed);
            }
            Occupied(mut entry) => {
                let res = entry.get_mut().blocks.try_send(header);
                if res.is_err() {
                    entry.remove();
                }
                res
            }
        }
    }
}
