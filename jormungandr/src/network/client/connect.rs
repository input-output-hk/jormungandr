use super::super::{
    grpc::{
        self,
        client::{BlockSubscription, FragmentSubscription, GossipSubscription},
    },
    p2p::{comm::PeerComms, Address},
    Channels, ConnectionState,
};
use super::{Client, ClientBuilder, GlobalStateR, InboundSubscriptions};
use crate::blockcfg::HeaderHash;
use chain_core::mempack::{self, ReadBuf, Readable};
use chain_network::data::block::BlockId;
use chain_network::error::{self as net_error, HandshakeError};

use futures03::channel::oneshot;
use futures03::future::BoxFuture;
use futures03::prelude::*;
use futures03::ready;

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
    let builder = Some(ClientBuilder {
        channels,
        logger: state.logger,
    });
    let cf = grpc::connect(&peer);
    let handle = ConnectHandle { receiver };
    let future = ConnectFuture {
        sender: Some(sender),
        builder,
        global: state.global.clone(),
        state: State::Connecting(Box::pin(cf)),
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

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("connection has been canceled")]
    Canceled,
    #[error("connection failed")]
    Connect(#[source] tonic::transport::Error),
    #[error("protocol handshake failed: {0}")]
    Handshake(#[source] HandshakeError),
    #[error("failed to decode genesis block in response")]
    DecodeBlock0(#[source] mempack::ReadError),
    #[error(
        "genesis block hash {peer_responded} reported by the peer is not the expected {expected}"
    )]
    Block0Mismatch {
        expected: HeaderHash,
        peer_responded: HeaderHash,
    },
    #[error("subscription request failed")]
    Subscription(#[source] net_error::Error),
    #[error("node address {peer_responded} reported by the peer is not the expected {expected}")]
    AddressMismatch {
        expected: Address,
        peer_responded: Address,
    },
}

enum State {
    // Establishing the protocol connection
    Connecting(BoxFuture<'static, Result<grpc::Client, grpc::ConnectError>>),
    BeforeHandshake,
    Handshake(BoxFuture<'static, Result<BlockId, HandshakeError>>),
    Subscribing(SubscriptionStaging),
    Done,
}

struct SubscriptionRequests {
    pub blocks: Option<BoxFuture<'static, Result<BlockSubscription, net_error::Error>>>,
    pub fragments: Option<BoxFuture<'static, Result<FragmentSubscription, net_error::Error>>>,
    pub gossip: Option<BoxFuture<'static, Result<GossipSubscription, net_error::Error>>>,
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
                .poll_canceled(cx)
            {
                return Err(ConnectError::Canceled).into();
            }

            let new_state = match self.state {
                State::Connecting(ref mut future) => {
                    let client =
                        ready!(Pin::new(&mut future).poll(cx)).map_err(ConnectError::Connect)?;
                    let handshake = client.handshake();
                    self.client = Some(client);
                    State::Handshake(Box::pin(handshake))
                }
                State::Handshake(ref mut future) => {
                    let block0 =
                        ready!(Pin::new(&mut future).poll(cx)).map_err(ConnectError::Handshake)?;
                    let mut buf = ReadBuf::from(block0.as_bytes());
                    let block0 = HeaderHash::read(&mut buf).map_err(ConnectError::DecodeBlock0)?;
                    self.match_block0(block0)?;
                    State::Subscribing(SubscriptionStaging::new())
                }
                State::Subscribing(ref mut staging) => {
                    let client = self.client.as_mut().expect("client must be connected");
                    match ready!(staging.poll_complete(client, cx))? {
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
                                Ok(()) => Ok(client).into(),
                                Err(_) => Err(ConnectError::Canceled).into(),
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
    pub node_id: Option<Address>,
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
                node_id: self
                    .node_id
                    .take()
                    .expect("remote node address should be known"),
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
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<InboundSubscriptions>, ConnectError>> {
        let mut ready = Poll::Pending;

        // Poll and resolve the request futures that are in progress
        drive_subscribe_request(cx, &mut self.req.blocks, &mut self.block_events, &mut ready)?;
        drive_subscribe_request(cx, &mut self.req.fragments, &mut self.fragments, &mut ready)?;
        drive_subscribe_request(cx, &mut self.req.gossip, &mut self.gossip, &mut ready)?;

        if let Some(inbound) = self.try_complete() {
            // All done
            return Ok(Some(inbound)).into();
        }

        // Initiate subscription requests.
        if !self.comms.block_announcements_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_block_announcements();
            self.req.blocks = Some(Box::pin(client.block_subscription(outbound)));
        }
        if !self.comms.fragments_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_fragments();
            self.req.fragments = Some(Box::pin(client.fragment_subscription(outbound)));
        }
        if !self.comms.gossip_subscribed() {
            ready = Poll::Ready(());
            let outbound = self.comms.subscribe_to_gossip();
            self.req.gossip = Some(Box::pin(client.gossip_subscription(outbound)));
        }

        // If progress was made, return Ready(Ok(None)) to call this again
        // for the next iteration.
        // Otherwise, return Pending to bubble up from the poll.
        ready.map(|()| Ok(None))
    }
}

fn drive_subscribe_request<R, S>(
    cx: &mut Context<'_>,
    req: &mut Option<R>,
    sub: &mut Option<S>,
    ready: &mut Poll<()>,
) -> Result<(), ConnectError>
where
    R: Future<Output = Result<S, net_error::Error>> + Unpin,
{
    if let Some(future) = req {
        let polled = Pin::new(future)
            .poll(cx)
            .map_err(ConnectError::Subscription)?;
        match polled {
            Poll::Pending => {}
            Poll::Ready(stream) => {
                *req = None;
                *sub = Some(stream);
                *ready = Poll::Ready(());
            }
        }
    }
    Ok(().into())
}
