use super::{
    buffer_sizes,
    p2p::comm::{
        BlockEventSubscription, FragmentSubscription, GossipSubscription, LockServerComms,
    },
    p2p::{Gossip as NodeData, Id},
    GlobalStateR,
};
use crate::{
    blockcfg::{Fragment, Header},
    intercom::{BlockMsg, TransactionMsg},
    settings::start::network::Configuration,
    utils::async_msg::{self, MessageBox},
};
use chain_network::data::gossip::Gossip;
use chain_network::error as net_error;
use jormungandr_lib::interfaces::FragmentOrigin;

use futures03::prelude::*;
use slog::Logger;

use std::fmt::Debug;
use std::task::{Context, Poll};

#[must_use = "`ServeBlockEvents` needs to be plugged into a service trait implementation"]
pub struct ServeBlockEvents<In> {
    inbound: Option<In>,
    lock: LockServerComms,
    logger: Logger,
}

impl<In> ServeBlockEvents<In> {
    pub(super) fn new(inbound: In, lock: LockServerComms, logger: Logger) -> Self {
        ServeBlockEvents {
            inbound: Some(inbound),
            lock,
            logger,
        }
    }
}

impl<In> Future for ServeBlockEvents<In> {
    type Output = Result<Subscription<In, BlockEventSubscription>, net_error::Error>;

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let polled_outbound = self
            .lock
            .poll_subscribe_with(|comms| comms.subscribe_to_block_events());
        Ok(polled_outbound.map(|outbound| {
            let inbound = self.inbound.take().expect("future polled after finish");
            Subscription::new(inbound, outbound, self.logger.clone())
        }))
    }
}

#[must_use = "`ServeGossip` needs to be plugged into a service trait implementation"]
pub struct ServeFragments<In> {
    inbound: Option<In>,
    lock: LockServerComms,
    logger: Logger,
}

impl<In> ServeFragments<In> {
    pub(super) fn new(inbound: In, lock: LockServerComms, logger: Logger) -> Self {
        ServeFragments {
            inbound: Some(inbound),
            lock,
            logger,
        }
    }
}

impl<In> Future for ServeFragments<In> {
    type Item = Subscription<In, FragmentSubscription>;
    type Error = net_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let polled_outbound = self
            .lock
            .poll_subscribe_with(|comms| comms.subscribe_to_fragments());
        Ok(polled_outbound.map(|outbound| {
            let inbound = self.inbound.take().expect("future polled after finish");
            Subscription::new(inbound, outbound, self.logger.clone())
        }))
    }
}

#[must_use = "`ServeGossip` needs to be plugged into a service trait implementation"]
pub struct ServeGossip<In> {
    inbound: Option<In>,
    lock: LockServerComms,
    logger: Logger,
}

impl<In> ServeGossip<In> {
    pub(super) fn new(inbound: In, lock: LockServerComms, logger: Logger) -> Self {
        ServeGossip {
            inbound: Some(inbound),
            lock,
            logger,
        }
    }
}

impl<In> Future for ServeGossip<In> {
    type Item = Subscription<In, GossipSubscription>;
    type Error = net_error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let polled_outbound = self
            .lock
            .poll_subscribe_with(|comms| comms.subscribe_to_gossip());
        Ok(polled_outbound.map(|outbound| {
            let inbound = self.inbound.take().expect("future polled after finish");
            Subscription::new(inbound, outbound, self.logger.clone())
        }))
    }
}

#[must_use = "`Subscription` needs to be plugged into a service trait implementation"]
pub struct Subscription<In, Out> {
    inbound: In,
    outbound: Out,
    logger: Logger,
}

impl<In, Out> Subscription<In, Out> {
    pub fn new(inbound: In, outbound: Out, logger: Logger) -> Self {
        Subscription {
            inbound,
            outbound,
            logger,
        }
    }
}

impl<In, Out> Stream for Subscription<In, Out>
where
    Out: Stream<Error = net_error::Error>,
    Out::Item: Debug,
{
    type Item = Out::Item;
    type Error = net_error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.outbound.poll() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(Some(item))) => Ok(Some(item).into()),
            Ok(Async::Ready(None)) => {
                debug!(
                    self.logger,
                    "subscription stream closed";
                    "direction" => "out",
                );
                Ok(None.into())
            }
            Err(e) => {
                debug!(
                    self.logger,
                    "subscription stream failed";
                    "error" => ?e,
                    "direction" => "out",
                );
                Err(e)
            }
        }
    }
}

impl<In, Out> Sink for Subscription<In, Out>
where
    In: Sink<SinkError = net_error::Error>,
{
    type SinkItem = In::SinkItem;
    type SinkError = net_error::Error;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        // Not logging the item here because start_send might refuse to send it
        // and it will end up logged redundantly. This won't be a problem with
        // futures 0.3.
        self.inbound.start_send(item)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        self.inbound.poll_complete().map_err(|err| {
            debug!(
                self.logger,
                "subscription sink failed";
                "error" => ?err,
                "direction" => "in",
            );
            err
        })
    }

    fn close(&mut self) -> Poll<(), Self::SinkError> {
        match self.inbound.close() {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(())) => {
                debug!(
                    self.logger,
                    "subscription stream closed";
                    "direction" => "in",
                );
                Ok(Async::Ready(()))
            }
            Err(e) => {
                warn!(
                    self.logger,
                    "failed to close processing sink for subscription";
                    "error" => ?e,
                    "direction" => "in",
                );
                Err(e)
            }
        }
    }
}

impl<In, Out> MapResponse for Subscription<In, Out> {
    type Response = ();
    type ResponseFuture = FutureResult<(), net_error::Error>;

    fn on_stream_termination(&mut self, res: Result<(), ProcessingError>) -> Self::ResponseFuture {
        match res {
            Ok(()) => {
                debug!(
                    self.logger,
                    "inbound subscription stream terminated by the peer";
                    "direction" => "in",
                );
                future::ok(())
            }
            Err(e) => {
                debug!(
                    self.logger,
                    "inbound subscription stream failed";
                    "error" => ?e,
                    "direction" => "in",
                );
                future::err(net_error::Error::new(
                    net_error::Code::Canceled,
                    "closed due to inbound stream failure",
                ))
            }
        }
    }
}

fn filter_gossip_node(node: &NodeData, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}

#[must_use = "sinks do nothing unless polled"]
pub struct BlockAnnouncementProcessor {
    mbox: MessageBox<BlockMsg>,
    node_id: Id,
    global_state: GlobalStateR,
    logger: Logger,
}

impl BlockAnnouncementProcessor {
    pub fn new(
        mbox: MessageBox<BlockMsg>,
        node_id: Id,
        global_state: GlobalStateR,
        logger: Logger,
    ) -> Self {
        BlockAnnouncementProcessor {
            mbox,
            node_id,
            global_state,
            logger,
        }
    }

    pub fn message_box(&self) -> MessageBox<BlockMsg> {
        self.mbox.clone()
    }

    fn mbox_error<T>(&self, err: async_msg::SendError<T>) -> net_error::Error
    where
        T: Send + Sync + 'static,
    {
        error!(
            self.logger,
            "failed to send block announcement to the block task";
            "reason" => %err,
        );
        net_error::Error::new(net_error::Code::Internal, err)
    }

    fn refresh_stat(&self) {
        let refresh_logger = self.logger.clone();
        self.global_state.spawn(
            self.global_state
                .peers
                .refresh_peer_on_block(self.node_id)
                .and_then(move |refreshed| {
                    if !refreshed {
                        debug!(
                            refresh_logger,
                            "received block from node that is not in the peer map",
                        );
                    }
                    Ok(())
                }),
        );
    }
}

#[must_use = "sinks do nothing unless polled"]
pub struct FragmentProcessor {
    mbox: MessageBox<TransactionMsg>,
    node_id: Id,
    global_state: GlobalStateR,
    logger: Logger,
    buffered_fragments: Vec<Fragment>,
}

impl FragmentProcessor {
    pub fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: Id,
        global_state: GlobalStateR,
        logger: Logger,
    ) -> Self {
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            logger,
            buffered_fragments: Vec::new(),
        }
    }

    fn refresh_stat(&self) {
        let refresh_logger = self.logger.clone();
        self.global_state.spawn(
            self.global_state
                .peers
                .refresh_peer_on_fragment(self.node_id)
                .and_then(move |refreshed| {
                    if !refreshed {
                        debug!(
                            refresh_logger,
                            "received fragment from node that is not in the peer map",
                        );
                    }
                    Ok(())
                }),
        );
    }
}

pub struct GossipProcessor {
    node_id: Id,
    global_state: GlobalStateR,
    logger: Logger,
}

impl GossipProcessor {
    pub fn new(node_id: Id, global_state: GlobalStateR, logger: Logger) -> Self {
        GossipProcessor {
            node_id,
            global_state,
            logger,
        }
    }

    pub fn process_item(&self, gossip: Gossip<NodeData>) {
        let (nodes, filtered_out): (Vec<_>, Vec<_>) = gossip.into_nodes().partition(|node| {
            filter_gossip_node(node, &self.global_state.config)
                || (node.id() == self.node_id && node.address().is_none())
        });
        if filtered_out.len() > 0 {
            debug!(self.logger, "nodes dropped from gossip: {:?}", filtered_out);
        }
        let refresh_logger = self.logger.clone();
        self.global_state.spawn(
            self.global_state
                .peers
                .refresh_peer_on_gossip(self.node_id)
                .and_then(move |refreshed| {
                    if !refreshed {
                        debug!(
                            refresh_logger,
                            "received gossip from node that is not in the peer map",
                        );
                    }
                    Ok(())
                }),
        );
        self.global_state.spawn(
            self.global_state
                .topology
                .accept_gossips(self.node_id, nodes.into()),
        );
    }
}

impl Sink for BlockAnnouncementProcessor {
    type SinkItem = Header;
    type SinkError = net_error::Error;

    fn start_send(&mut self, header: Header) -> StartSend<Header, net_error::Error> {
        let polled_ready = self.mbox.poll_ready().map_err(|e| self.mbox_error(e))?;
        if polled_ready.is_not_ready() {
            return Ok(AsyncSink::NotReady(header));
        }
        let polled = self
            .mbox
            .start_send(BlockMsg::AnnouncedBlock(header, self.node_id))
            .map_err(|e| self.mbox_error(e))?;
        match polled {
            AsyncSink::Ready => {
                self.refresh_stat();
                Ok(AsyncSink::Ready)
            }
            AsyncSink::NotReady(BlockMsg::AnnouncedBlock(header, _)) => {
                Ok(AsyncSink::NotReady(header))
            }
            AsyncSink::NotReady(_) => unreachable!(),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), net_error::Error> {
        self.mbox.poll_complete().map_err(|e| {
            error!(
                self.logger,
                "communication channel to the block task failed";
                "reason" => %e,
            );
            net_error::Error::new(net_error::Code::Internal, e)
        })
    }

    fn close(&mut self) -> Poll<(), net_error::Error> {
        self.mbox.close().map_err(|e| {
            warn!(
                self.logger,
                "failed to close communication channel to the block task";
                "reason" => %e,
            );
            net_error::Error::new(net_error::Code::Internal, e)
        })
    }
}

impl Sink for FragmentProcessor {
    type SinkItem = Fragment;
    type SinkError = net_error::Error;

    fn start_send(&mut self, fragment: Fragment) -> StartSend<Fragment, net_error::Error> {
        if self.buffered_fragments.len() >= buffer_sizes::inbound::FRAGMENTS {
            return Ok(AsyncSink::NotReady(fragment));
        }
        self.buffered_fragments.push(fragment);
        let async_send = self.try_send_fragments()?;
        Ok(async_send.map(|()| self.buffered_fragments.pop().unwrap()))
    }

    fn poll_complete(&mut self) -> Poll<(), net_error::Error> {
        if self.buffered_fragments.is_empty() {
            self.mbox.poll_complete().map_err(|e| {
                error!(
                    self.logger,
                    "communication channel to the fragment task failed";
                    "reason" => %e,
                );
                net_error::Error::new(net_error::Code::Internal, e)
            })
        } else {
            match self.try_send_fragments()? {
                AsyncSink::Ready => Ok(Async::Ready(())),
                AsyncSink::NotReady(()) => Ok(Async::NotReady),
            }
        }
    }

    fn close(&mut self) -> Poll<(), net_error::Error> {
        self.mbox.close().map_err(|e| {
            warn!(
                self.logger,
                "failed to close communication channel to the fragment task";
                "reason" => %e,
            );
            net_error::Error::new(net_error::Code::Internal, e)
        })
    }
}

impl FragmentProcessor {
    fn try_send_fragments(&mut self) -> Result<AsyncSink<()>, net_error::Error> {
        let fragments = self.buffered_fragments.split_off(0);
        let polled = self
            .mbox
            .start_send(TransactionMsg::SendTransaction(
                FragmentOrigin::Network,
                fragments,
            ))
            .map_err(|e| {
                error!(
                    self.logger,
                    "failed to send fragments to the fragment task";
                    "reason" => %e,
                );
                net_error::Error::new(net_error::Code::Internal, e)
            })?;
        match polled {
            AsyncSink::Ready => {
                self.refresh_stat();
                Ok(AsyncSink::Ready)
            }
            AsyncSink::NotReady(TransactionMsg::SendTransaction(_, fragments)) => {
                self.buffered_fragments = fragments;
                Ok(AsyncSink::NotReady(()))
            }
            AsyncSink::NotReady(_) => unreachable!(),
        }
    }
}

impl Sink for GossipProcessor {
    type SinkItem = Gossip<NodeData>;
    type SinkError = net_error::Error;

    fn start_send(
        &mut self,
        gossip: Gossip<NodeData>,
    ) -> StartSend<Self::SinkItem, net_error::Error> {
        self.process_item(gossip);
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), net_error::Error> {
        Ok(Async::Ready(()))
    }
}
