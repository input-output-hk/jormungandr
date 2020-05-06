use super::{
    buffer_sizes,
    convert::Decode,
    p2p::{Address, Gossip},
    GlobalStateR,
};
use crate::{
    blockcfg::Fragment,
    intercom::{BlockMsg, TransactionMsg},
    settings::start::network::Configuration,
    utils::async_msg::{self, MessageBox},
};
use chain_network::data as net_data;
use chain_network::error::{Code, Error};
use jormungandr_lib::interfaces::FragmentOrigin;

use futures03::future::BoxFuture;
use futures03::prelude::*;
use futures03::ready;
use slog::Logger;

use std::pin::Pin;
use std::task::{Context, Poll};

fn filter_gossip_node(node: &Gossip, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}

fn handle_mbox_error(err: async_msg::SendError, logger: &Logger) -> Error {
    error!(
        logger,
        "failed to send block announcement to the block task";
        "reason" => %err,
    );
    Error::new(Code::Internal, err)
}

pub async fn process_block_announcements<S>(
    stream: S,
    mbox: MessageBox<BlockMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Header, Error = Error>,
{
    let sink = BlockAnnouncementProcessor::new(mbox, node_id, global_state, logger.clone());
    stream
        .into_stream()
        .forward(sink)
        .await
        .unwrap_or_else(|e| {
            debug!(logger, "processing of inbound subscription stream failed"; "error" => ?e);
        });
}

pub async fn process_gossip<S>(
    stream: S,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Gossip, Error = Error>,
{
    let processor = GossipProcessor::new(node_id, global_state, logger.clone());
    stream
        .into_stream()
        .forward(processor)
        .await
        .unwrap_or_else(|e| {
            debug!(
                logger,
                "processing of inbound gossip failed";
                "error" => ?e,
            );
        });
}

pub async fn process_fragments<S>(
    stream: S,
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
) where
    S: TryStream<Ok = net_data::Fragment, Error = Error>,
{
    let sink = FragmentProcessor::new(mbox, node_id, global_state, logger.clone());
    stream
        .into_stream()
        .forward(sink)
        .await
        .unwrap_or_else(|e| {
            debug!(logger, "processing of inbound subscription stream failed"; "error" => ?e);
        });
}

#[must_use = "sinks do nothing unless polled"]
pub struct BlockAnnouncementProcessor {
    mbox: MessageBox<BlockMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
    pending_processing: PendingProcessing,
}

impl BlockAnnouncementProcessor {
    pub(super) fn new(
        mbox: MessageBox<BlockMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        logger: Logger,
    ) -> Self {
        BlockAnnouncementProcessor {
            mbox,
            node_id,
            global_state,
            logger,
            pending_processing: PendingProcessing::default(),
        }
    }

    pub fn message_box(&self) -> MessageBox<BlockMsg> {
        self.mbox.clone()
    }

    fn refresh_stat(&mut self) {
        let refresh_logger = self.logger.clone();
        let state = self.global_state.clone();
        let node_id = self.node_id.clone();
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_block(node_id).await;
            if !refreshed {
                debug!(
                    refresh_logger,
                    "received block from node that is not in the peer map",
                );
            }
        };
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.pending_processing.start(fut);
    }

    fn poll_flush_mbox(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.mbox)
            .poll_flush(cx)
            .map_err(|e| handle_mbox_error(e, &self.logger))
    }
}

#[must_use = "sinks do nothing unless polled"]
pub struct FragmentProcessor {
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
    buffered_fragments: Vec<Fragment>,
    pending_processing: PendingProcessing,
}

impl FragmentProcessor {
    pub(super) fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        logger: Logger,
    ) -> Self {
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            logger,
            buffered_fragments: Vec::new(),
            pending_processing: PendingProcessing::default(),
        }
    }

    fn refresh_stat(&mut self) {
        let refresh_logger = self.logger.clone();
        let state = self.global_state.clone();
        let node_id = self.node_id.clone();
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_fragment(node_id).await;
            if !refreshed {
                debug!(
                    refresh_logger,
                    "received fragment from node that is not in the peer map",
                );
            }
        };
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.pending_processing.start(fut);
    }
}

pub struct GossipProcessor {
    node_id: Address,
    global_state: GlobalStateR,
    logger: Logger,
    pending_processing: PendingProcessing,
}

impl GossipProcessor {
    pub(super) fn new(node_id: Address, global_state: GlobalStateR, logger: Logger) -> Self {
        GossipProcessor {
            node_id,
            global_state,
            logger,
            pending_processing: Default::default(),
        }
    }
}

impl Sink<net_data::Header> for BlockAnnouncementProcessor {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                match self.as_mut().poll_flush_mbox(cx) {
                    Poll::Ready(res) => res?,
                    Poll::Pending => (),
                }
                Poll::Pending
            }
            Poll::Ready(()) => self
                .mbox
                .poll_ready(cx)
                .map_err(|e| handle_mbox_error(e, &self.logger)),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, raw_header: net_data::Header) -> Result<(), Error> {
        let header = raw_header.decode()?;
        let node_id = self.node_id.clone();
        self.mbox
            .start_send(BlockMsg::AnnouncedBlock(header, node_id))
            .map_err(|e| handle_mbox_error(e, &self.logger))?;
        self.refresh_stat();
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                match self.as_mut().poll_flush_mbox(cx) {
                    Poll::Ready(res) => res?,
                    Poll::Pending => (),
                };
                Poll::Pending
            }
            Poll::Ready(()) => self.as_mut().poll_flush_mbox(cx),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                match self.as_mut().poll_flush_mbox(cx) {
                    Poll::Ready(res) => res?,
                    Poll::Pending => (),
                };
                Poll::Pending
            }
            Poll::Ready(()) => Pin::new(&mut self.mbox)
                .poll_close(cx)
                .map_err(|e| handle_mbox_error(e, &self.logger)),
        }
    }
}

impl Sink<net_data::Fragment> for FragmentProcessor {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if self.buffered_fragments.len() >= buffer_sizes::inbound::FRAGMENTS {
            ready!(self.poll_send_fragments(cx))?;
            debug_assert!(self.buffered_fragments.is_empty());
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, raw_fragment: net_data::Fragment) -> Result<(), Error> {
        assert!(
            self.buffered_fragments.len() < buffer_sizes::inbound::FRAGMENTS,
            "should call `poll_ready` which returns `Poll::Ready(Ok(()))` before `start_send`",
        );
        let fragment = raw_fragment.decode()?;
        self.buffered_fragments.push(fragment);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if self.buffered_fragments.is_empty() {
                match self.poll_complete_refresh_stat(cx) {
                    _ => (),
                };
                return Pin::new(&mut self.mbox).poll_flush(cx).map_err(|e| {
                    error!(
                        self.logger,
                        "communication channel to the fragment task failed";
                        "reason" => %e,
                    );
                    Error::new(Code::Internal, e)
                });
            } else {
                ready!(self.poll_send_fragments(cx))?;
            }
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if self.buffered_fragments.is_empty() {
                ready!(self.poll_complete_refresh_stat(cx));
                return Pin::new(&mut self.mbox).poll_close(cx).map_err(|e| {
                    warn!(
                        self.logger,
                        "failed to close communication channel to the fragment task";
                        "reason" => %e,
                    );
                    Error::new(Code::Internal, e)
                });
            } else {
                ready!(self.poll_send_fragments(cx))?;
            }
        }
    }
}

impl FragmentProcessor {
    fn poll_send_fragments(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let logger = &self.logger;
        ready!(self.mbox.poll_ready(cx)).map_err(|e| {
            debug!(logger, "error sending fragments for processing"; "reason" => %e);
            Error::new(Code::Internal, e)
        })?;
        let fragments = self.buffered_fragments.split_off(0);
        self.mbox
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
                Error::new(Code::Internal, e)
            })?;
        self.refresh_stat();
        Poll::Ready(Ok(()))
    }

    fn poll_complete_refresh_stat(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.pending_processing.poll_complete(cx)
    }
}

impl Sink<net_data::Gossip> for GossipProcessor {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        ready!(self.pending_processing.poll_complete(cx));
        Ok(()).into()
    }

    fn start_send(mut self: Pin<&mut Self>, gossip: net_data::Gossip) -> Result<(), Error> {
        let nodes = gossip.nodes.decode()?;
        let (nodes, filtered_out): (Vec<_>, Vec<_>) = nodes.into_iter().partition(|node| {
            filter_gossip_node(node, &self.global_state.config) || node.address().is_none()
        });
        if filtered_out.len() > 0 {
            debug!(self.logger, "nodes dropped from gossip: {:?}", filtered_out);
        }
        let node_id1 = self.node_id.clone();
        let node_id2 = self.node_id.clone();
        let state1 = self.global_state.clone();
        let state2 = self.global_state.clone();
        let logger = self.logger.clone();
        let fut = future::join(
            async move {
                let refreshed = state1.peers.refresh_peer_on_gossip(node_id1).await;
                if !refreshed {
                    debug!(
                        logger,
                        "received gossip from node that is not in the peer map",
                    );
                }
            },
            async move {
                state2.topology.accept_gossips(node_id2, nodes.into()).await;
            },
        )
        .map(|_| ());
        self.pending_processing.start(fut);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        ready!(self.pending_processing.poll_complete(cx));
        Ok(()).into()
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        ready!(self.pending_processing.poll_complete(cx));
        Ok(()).into()
    }
}

#[derive(Default)]
struct PendingProcessing(Option<BoxFuture<'static, ()>>);

impl PendingProcessing {
    fn start(&mut self, future: impl Future<Output = ()> + Send + 'static) {
        self.0 = Some(future.boxed());
    }

    fn poll_complete(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(fut) = &mut self.0 {
            ready!(Pin::new(fut).poll(cx));
            self.0 = None;
        }
        Poll::Ready(())
    }
}
