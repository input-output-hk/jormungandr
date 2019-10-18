use super::super::{
    grpc,
    p2p::{comm::PeerComms, topology},
    Channels, ConnectionState,
};
use super::{Client, ClientBuilder, GlobalStateR};
use crate::blockcfg::{Block, Fragment, HeaderHash};
use network_core::client::{self as core_client};
use network_core::client::{BlockService, FragmentService, GossipService, P2pService};
use network_core::error as core_error;
use network_core::gossip::Node as _;

use futures::prelude::*;
use futures::sync::oneshot;
use thiserror::Error;

use std::error;
use std::mem;

/// Initiates a client connection, returning a connection handle and
/// the connection future that must be polled to complete the connection.
///
/// Note that this is the only function in this module that is tied to the
/// gRPC protocol, all other code is generic in terms of network-core traits.
/// This is intentional, to facilitate extension to different protocols
/// in the future.
pub fn connect(
    state: ConnectionState,
    channels: Channels,
) -> (ConnectHandle, ConnectFuture<grpc::ConnectFuture>) {
    let (sender, receiver) = oneshot::channel();
    let addr = state.connection;
    let node_id = state.global.topology.node().id();
    let builder = Some(ClientBuilder {
        channels,
        logger: state.logger,
    });
    let cf = grpc::connect(addr, Some(node_id), state.global.executor.clone());
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
pub struct ConnectFuture<F>
where
    F: Future,
    F::Item: BlockService + FragmentService + GossipService,
{
    sender: Option<oneshot::Sender<PeerComms>>,
    builder: Option<ClientBuilder>,
    global: GlobalStateR,
    client: Option<F::Item>,
    state: State<F>,
}

#[derive(Error, Debug)]
pub enum ConnectError<E>
where
    E: error::Error + 'static,
{
    #[error("connection has been canceled")]
    Canceled,
    #[error("connection failed: {source}")]
    Connect { source: E },
    #[error("client connection unable to send requests")]
    ClientNotReady { source: core_error::Error },
    #[error("protocol handshake failed: {source}")]
    Handshake { source: core_client::HandshakeError },
    #[error(
        "genesis block hash {peer_responded} reported by the peer is not the expected {expected}"
    )]
    Block0Mismatch {
        expected: HeaderHash,
        peer_responded: HeaderHash,
    },
    #[error("subscription request failed")]
    Subscription { source: core_error::Error },
    #[error(
        "node identifier {peer_responded} reported by the peer is not the expected {expected}"
    )]
    NodeIdMismatch {
        expected: topology::NodeId,
        peer_responded: topology::NodeId,
    },
}

enum State<F>
where
    F: Future,
    F::Item: BlockService + FragmentService + GossipService,
{
    // Establishing the protocol connection
    Connecting(F),
    BeforeHandshake,
    Handshake(<F::Item as BlockService>::HandshakeFuture),
    Subscribing {
        req: SubscriptionRequests<F::Item>,
        sub: InboundSubscriptionStaging<F::Item>,
        comms: PeerComms,
    },
    Done,
}

struct SubscriptionRequests<T>
where
    T: BlockService + FragmentService + GossipService,
{
    pub blocks: Option<<T as BlockService>::BlockSubscriptionFuture>,
    pub fragments: Option<<T as FragmentService>::FragmentSubscriptionFuture>,
    pub gossip: Option<<T as GossipService>::GossipSubscriptionFuture>,
}

impl<T> SubscriptionRequests<T>
where
    T: BlockService + FragmentService + GossipService,
{
    fn new() -> Self {
        SubscriptionRequests {
            blocks: None,
            fragments: None,
            gossip: None,
        }
    }
}

pub struct InboundSubscriptions<T>
where
    T: BlockService + FragmentService + GossipService,
{
    pub node_id: topology::NodeId,
    pub block_events: <T as BlockService>::BlockSubscription,
    pub fragments: <T as FragmentService>::FragmentSubscription,
    pub gossip: <T as GossipService>::GossipSubscription,
}

struct InboundSubscriptionStaging<T>
where
    T: BlockService + FragmentService + GossipService,
{
    pub node_id: Option<topology::NodeId>,
    pub block_events: Option<<T as BlockService>::BlockSubscription>,
    pub fragments: Option<<T as FragmentService>::FragmentSubscription>,
    pub gossip: Option<<T as GossipService>::GossipSubscription>,
}

impl<T> InboundSubscriptionStaging<T>
where
    T: BlockService + FragmentService + GossipService,
{
    fn new() -> Self {
        InboundSubscriptionStaging {
            node_id: None,
            block_events: None,
            fragments: None,
            gossip: None,
        }
    }

    fn try_complete(&mut self) -> Option<InboundSubscriptions<T>> {
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

fn poll_client_ready<T, E>(client: &mut T) -> Poll<(), ConnectError<E>>
where
    T: core_client::Client,
    E: error::Error + 'static,
{
    client
        .poll_ready()
        .map_err(|e| ConnectError::ClientNotReady { source: e })
}

impl<F> Future for ConnectFuture<F>
where
    F: Future,
    F::Error: error::Error + 'static,
    F::Item: core_client::Client,
    F::Item: P2pService<NodeId = topology::NodeId>,
    F::Item: BlockService<Block = Block>,
    F::Item: FragmentService<Fragment = Fragment>,
    F::Item: GossipService<Node = topology::NodeData>,
    <F::Item as BlockService>::UploadBlocksFuture: Send + 'static,
    <F::Item as FragmentService>::FragmentSubscription: Send + 'static,
    <F::Item as GossipService>::GossipSubscription: Send + 'static,
{
    type Item = Client<F::Item>;
    type Error = ConnectError<F::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            // First, check if the connection is cancelled
            if let Async::Ready(()) = self
                .sender
                .as_mut()
                .expect("polled a future after it has been resolved")
                .poll_cancel()
                .unwrap()
            {
                return Err(ConnectError::Canceled);
            }

            let new_state = match self.state {
                State::Connecting(ref mut future) => {
                    let client = try_ready!(future
                        .poll()
                        .map_err(|e| ConnectError::Connect { source: e }));
                    self.client = Some(client);
                    State::BeforeHandshake
                }
                State::BeforeHandshake => {
                    let client = self.client.as_mut().expect("client must be connected");
                    try_ready!(poll_client_ready(client));
                    State::Handshake(client.handshake())
                }
                State::Handshake(ref mut future) => {
                    let block0 = try_ready!(future
                        .poll()
                        .map_err(|e| ConnectError::Handshake { source: e }));
                    self.match_block0(block0)?;
                    State::Subscribing {
                        req: SubscriptionRequests::new(),
                        sub: InboundSubscriptionStaging::new(),
                        comms: PeerComms::new(),
                    }
                }
                State::Subscribing {
                    ref mut req,
                    ref mut sub,
                    ref mut comms,
                } => {
                    let client = self.client.as_mut().expect("client must be connected");
                    match try_ready!(poll_subscribe(client, req, sub, comms)) {
                        None => continue,
                        Some(inbound) => {
                            // After subscribing is complete, set up the client and
                            // send its communication handles to be received by
                            // ClientHandle::try_complete().
                            let mut comms = if let State::Subscribing { comms, .. } =
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

impl<F> ConnectFuture<F>
where
    F: Future,
    F::Error: error::Error + 'static,
    F::Item: BlockService + FragmentService + GossipService,
{
    fn match_block0(&self, peer_responded: HeaderHash) -> Result<(), ConnectError<F::Error>> {
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

fn poll_subscribe<T, E>(
    client: &mut T,
    req: &mut SubscriptionRequests<T>,
    sub: &mut InboundSubscriptionStaging<T>,
    comms: &mut PeerComms,
) -> Poll<Option<InboundSubscriptions<T>>, ConnectError<E>>
where
    E: error::Error + 'static,
    T: core_client::Client,
    T: P2pService<NodeId = topology::NodeId>,
    T: BlockService<Block = Block>,
    T: FragmentService<Fragment = Fragment>,
    T: GossipService<Node = topology::NodeData>,
{
    // Poll and resolve the request futures that are in progress
    let not_ready_1 = drive_subscription(&mut req.blocks, &mut sub.block_events, &mut sub.node_id)?;
    let not_ready_2 = drive_subscription(&mut req.fragments, &mut sub.fragments, &mut sub.node_id)?;
    let not_ready_3 = drive_subscription(&mut req.gossip, &mut sub.gossip, &mut sub.node_id)?;

    if not_ready_1 && not_ready_2 && not_ready_3 {
        return Ok(Async::NotReady);
    }

    if let Some(inbound) = sub.try_complete() {
        // All done
        return Ok(Some(inbound).into());
    }

    // Make subscription requests if the client is ready
    if !comms.block_announcements_subscribed() {
        try_ready!(poll_client_ready(client));
        let outbound = comms.subscribe_to_block_announcements();
        req.blocks = Some(client.block_subscription(outbound));
    }
    if !comms.fragments_subscribed() {
        try_ready!(poll_client_ready(client));
        let outbound = comms.subscribe_to_fragments();
        req.fragments = Some(client.fragment_subscription(outbound));
    }
    if !comms.gossip_subscribed() {
        try_ready!(poll_client_ready(client));
        let outbound = comms.subscribe_to_gossip();
        req.gossip = Some(client.gossip_subscription(outbound));
    }

    // Call this again for the next iteration
    Ok(None.into())
}

fn drive_subscription<R, S, E>(
    req: &mut Option<R>,
    sub: &mut Option<S>,
    discovered_node_id: &mut Option<topology::NodeId>,
) -> Result<bool, ConnectError<E>>
where
    R: Future<Item = (S, topology::NodeId), Error = core_error::Error>,
    E: error::Error + 'static,
{
    if let Some(future) = req {
        let polled = future
            .poll()
            .map_err(|e| ConnectError::Subscription { source: e })?;
        match polled {
            Async::Ready((stream, node_id)) => {
                *req = None;
                handle_subscription_node_id(discovered_node_id, node_id)?;
                *sub = Some(stream);
                Ok(false)
            }
            Async::NotReady => Ok(true),
        }
    } else {
        Ok(false)
    }
}

fn handle_subscription_node_id<E>(
    staged: &mut Option<topology::NodeId>,
    node_id: topology::NodeId,
) -> Result<(), ConnectError<E>>
where
    E: error::Error + 'static,
{
    match *staged {
        None => {
            *staged = Some(node_id);
        }
        Some(expected) => {
            if node_id != expected {
                return Err(ConnectError::NodeIdMismatch {
                    expected,
                    peer_responded: node_id,
                });
            }
        }
    }
    Ok(())
}
