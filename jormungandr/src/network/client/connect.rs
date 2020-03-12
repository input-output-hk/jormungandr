use super::super::{
    grpc::{
        self,
        client::{BlockSubscription, FragmentSubscription, GossipSubscription},
    },
    p2p::{comm::PeerComms, Id},
    Channels, ConnectionState,
};
use super::{Client, ClientBuilder, GlobalStateR, InboundSubscriptions};
use crate::blockcfg::HeaderHash;
use chain_network::data::block::BlockId;
use chain_network::error::{self as net_error, HandshakeError};

use futures03::channel::oneshot;
use futures03::future::BoxFuture;
use futures03::prelude::*;
use thiserror::Error;

use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Initiates a client connection, returning a connection handle and
/// the connection future that must be polled to complete the connection.
///
/// Note that this is the only function in this module that is tied to the
/// gRPC protocol, all other code is generic in terms of network-core traits.
/// This is intentional, to facilitate extension to different protocols
/// in the future.
pub fn connect(state: ConnectionState, channels: Channels) -> (ConnectHandle, ConnectFuture) {
    let (sender, receiver) = oneshot::channel();
    let peer = state.peer();
    let node_id = state.global.topology.node_id();
    let builder = Some(ClientBuilder {
        channels,
        logger: state.logger,
    });
    let cf = grpc::connect(&peer, Some(node_id), state.global.executor.clone());
    let handle = ConnectHandle { receiver };
    let future = ConnectFuture {
        sender: Some(sender),
        builder,
        global: state.global.clone(),
        state: State::Connecting(cf),
        client: None,
    };
    (handle, future)
}

/// Handle used to monitor the P2P client in process of
/// establishing a connection and subscription streams.
///
/// If the handle is dropped before the connection is established,
/// the client connection is canceled.
pub struct ConnectHandle {
    receiver: oneshot::Receiver<PeerComms>,
}

/// An error type to signal that the connection was not established.
/// The reason should be logged already, so this error type should not be
/// used for reporting.
pub type ConnectCanceled = oneshot::Canceled;

impl ConnectHandle {
    /// Checks if the client has connected and established subscriptions,
    /// and if so, returns the communication handles.
    ///
    /// This method does not use a task context and does not schedule a wakeup.
    pub fn try_complete(&mut self) -> Result<Option<PeerComms>, ConnectCanceled> {
        self.receiver.try_recv()
    }
}

/// The future that drives P2P client to establish a connection.
#[must_use = "futures do nothing unless polled"]
pub struct ConnectFuture {
    sender: Option<oneshot::Sender<PeerComms>>,
    builder: Option<ClientBuilder>,
    global: GlobalStateR,
    client: Option<grpc::Client>,
    state: State,
}

#[derive(Error, Debug)]
pub enum ConnectError {
    #[error("connection has been canceled")]
    Canceled,
    #[error("connection failed")]
    Connect(#[source] tonic::transport::Error),
    #[error("client connection unable to send requests")]
    ClientNotReady(#[source] net_error::Error),
    #[error("protocol handshake failed: {0}")]
    Handshake(#[source] HandshakeError),
    #[error(
        "genesis block hash {peer_responded} reported by the peer is not the expected {expected}"
    )]
    Block0Mismatch {
        expected: HeaderHash,
        peer_responded: HeaderHash,
    },
    #[error("subscription request failed")]
    Subscription(#[source] net_error::Error),
    #[error(
        "node identifier {peer_responded} reported by the peer is not the expected {expected}"
    )]
    IdMismatch { expected: Id, peer_responded: Id },
}

enum State {
    // Establishing the protocol connection
    Connecting(BoxFuture<'static, grpc::Client>),
    BeforeHandshake,
    Handshake(BoxFuture<'static, Result<BlockId, HandshakeError>>),
    Subscribing(SubscriptionStaging),
    Done,
}

struct SubscriptionRequests {
    pub blocks: Option<BoxFuture<'static, BlockSubscription>>,
    pub fragments: Option<BoxFuture<'static, FragmentSubscription>>,
    pub gossip: Option<BoxFuture<'static, GossipSubscription>>,
}

impl SubscriptionRequests {
    fn new() -> Self {
        SubscriptionRequests {
            blocks: None,
            fragments: None,
            gossip: None,
        }
    }
}

impl Future for ConnectFuture {
    type Output = Result<Client, ConnectError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            // First, check if the connection is cancelled
            if let Poll::Ready(()) = self
                .sender
                .as_mut()
                .expect("polled a future after it has been resolved")
                .poll_canceled()
                .unwrap()
            {
                return Err(ConnectError::Canceled);
            }

            let new_state = match self.state {
                State::Connecting(ref mut future) => {
                    let client = try_ready!(future.poll(cx).map_err(ConnectError::Connect));
                    self.client = Some(client);
                    State::Handshake(Box::pin(self.client.handshake()))
                }
                State::Handshake(ref mut future) => {
                    let block0 = try_ready!(future.poll(cx).map_err(ConnectError::Handshake));
                    self.match_block0(block0)?;
                    State::Subscribing(SubscriptionStaging::new())
                }
                State::Subscribing(ref mut staging) => {
                    let client = self.client.as_mut().expect("client must be connected");
                    match try_ready!(staging.poll_complete(client)) {
                        None => continue,
                        Some(inbound) => {
                            // After subscribing is complete, set up the client and
                            // send its communication handles to be received by
                            // ClientHandle::try_complete().
                            let mut comms =
                                if let State::Subscribing(SubscriptionStaging { comms, .. }) =
                                    mem::replace(&mut self.state, State::Done)
                                {
                                    comms
                                } else {
                                    unreachable!()
                                };
                            let client = Client::new(
                                self.client.take().expect("client must be connected"),
                                self.builder.take().unwrap(),
                                self.global.clone(),
                                inbound,
                                &mut comms,
                            );
                            return match self.sender.take().unwrap().send(comms) {
                                Ok(()) => Ok(client.into()),
                                Err(_) => Err(ConnectError::Canceled),
                            };
                        }
                    }
                }
                State::Done => panic!("polled a future after it has been resolved"),
            };
            self.state = new_state;
        }
    }
}

impl ConnectFuture {
    fn match_block0(&self, peer_responded: HeaderHash) -> Result<(), ConnectError> {
        let expected = self.global.block0_hash;
        if expected == peer_responded {
            Ok(())
        } else {
            Err(ConnectError::Block0Mismatch {
                expected,
                peer_responded,
            })
        }
    }
}

struct SubscriptionStaging {
    pub node_id: Option<Id>,
    pub block_events: Option<BlockSubscription>,
    pub fragments: Option<FragmentSubscription>,
    pub gossip: Option<GossipSubscription>,
    pub req: SubscriptionRequests,
    pub comms: PeerComms,
}

impl SubscriptionStaging {
    fn new() -> Self {
        SubscriptionStaging {
            node_id: None,
            block_events: None,
            fragments: None,
            gossip: None,
            req: SubscriptionRequests::new(),
            comms: PeerComms::new(),
        }
    }

    fn try_complete(&mut self) -> Option<InboundSubscriptions> {
        match (&self.block_events, &self.fragments, &self.gossip) {
            (&Some(_), &Some(_), &Some(_)) => Some(InboundSubscriptions {
                node_id: self.node_id.take().expect("remote node ID should be known"),
                block_events: self.block_events.take().unwrap(),
                fragments: self.fragments.take().unwrap(),
                gossip: self.gossip.take().unwrap(),
            }),
            _ => None,
        }
    }
}

impl SubscriptionStaging {
    fn poll_complete(
        &mut self,
        client: &mut grpc::Client,
    ) -> Poll<Result<Option<InboundSubscriptions>, ConnectError>> {
        let mut ready = Poll::Pending;

        // Poll and resolve the request futures that are in progress
        drive_subscribe_request(
            &mut self.req.blocks,
            &mut self.block_events,
            &mut self.node_id,
            &mut ready,
        )?;
        drive_subscribe_request(
            &mut self.req.fragments,
            &mut self.fragments,
            &mut self.node_id,
            &mut ready,
        )?;
        drive_subscribe_request(
            &mut self.req.gossip,
            &mut self.gossip,
            &mut self.node_id,
            &mut ready,
        )?;

        if let Some(inbound) = self.try_complete() {
            // All done
            return Ok(Some(inbound).into());
        }

        // Initiate subscription requests.
        if !self.comms.block_announcements_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_block_announcements();
            self.req.blocks = Some(client.block_subscription(outbound));
        }
        if !self.comms.fragments_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_fragments();
            self.req.fragments = Some(client.fragment_subscription(outbound));
        }
        if !self.comms.gossip_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_gossip();
            self.req.gossip = Some(client.gossip_subscription(outbound));
        }

        // If progress was made, return Ready(None) to call this again
        // for the next iteration.
        // Otherwise, return NotReady to bubble up from the poll.
        Ok(ready.map(|()| None))
    }
}

fn drive_subscribe_request<R, S>(
    cx: &mut Context<'_>,
    req: &mut Option<R>,
    sub: &mut Option<S>,
    discovered_node_id: &mut Option<Id>,
    ready: &mut Poll<()>,
) -> Result<(), ConnectError>
where
    R: Future<Output = Result<(S, Id), net_error::Error>>,
{
    if let Some(future) = req {
        let polled = future.poll(cx).map_err(ConnectError::Subscription)?;
        match polled {
            Poll::Pending => {}
            Poll::Ready((stream, node_id)) => {
                *req = None;
                handle_subscription_node_id(discovered_node_id, node_id)?;
                *sub = Some(stream);
                *ready = Poll::Ready(());
            }
        }
    }
    Ok(().into())
}

fn handle_subscription_node_id(staged: &mut Option<Id>, node_id: Id) -> Result<(), ConnectError> {
    match *staged {
        None => {
            *staged = Some(node_id);
        }
        Some(expected) => {
            if node_id != expected {
                return Err(ConnectError::IdMismatch {
                    expected,
                    peer_responded: node_id,
                });
            }
        }
    }
    Ok(())
}
