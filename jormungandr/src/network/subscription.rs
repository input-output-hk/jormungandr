use super::{buffer_sizes, convert::Decode, p2p::Address, GlobalStateR};
use crate::{
    blockcfg::Fragment,
    intercom::{BlockMsg, TopologyMsg, TransactionMsg},
    settings::start::network::Configuration,
    topology::Gossip,
    utils::async_msg::{self, MessageBox},
};
use chain_network::data as net_data;
use chain_network::error::{Code, Error};
use jormungandr_lib::interfaces::FragmentOrigin;

use futures::future::BoxFuture;
use futures::prelude::*;
use futures::ready;
use tracing::Span;

use std::error::Error as _;
use std::mem;
use std::pin::Pin;
use std::task::{Context, Poll};
use tracing_futures::Instrument;

fn filter_gossip_node(node: &Gossip, config: &Configuration) -> bool {
    if config.allow_private_addresses {
        node.has_valid_address()
    } else {
        node.is_global()
    }
}

fn handle_mbox_error(err: async_msg::SendError) -> Error {
    tracing::error!(
        reason = %err,
        "failed to send block announcement to the block task"
    );
    Error::new(Code::Internal, err)
}

pub async fn process_block_announcements<S>(
    stream: S,
    mbox: MessageBox<BlockMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    span: Span,
) where
    S: TryStream<Ok = net_data::Header, Error = Error>,
{
    let sink = BlockAnnouncementProcessor::new(mbox, node_id, global_state, span);
    stream
        .into_stream()
        .forward(sink)
        .await
        .unwrap_or_else(|e| {
            tracing::debug!(error = ?e, "processing of inbound subscription stream failed");
        });
}

pub async fn process_gossip<S>(
    stream: S,
    mbox: MessageBox<TopologyMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    span: Span,
) where
    S: TryStream<Ok = net_data::Gossip, Error = Error>,
{
    let processor = GossipProcessor::new(mbox, node_id, global_state, span);
    stream
        .into_stream()
        .forward(processor)
        .await
        .unwrap_or_else(|e| {
            tracing::debug!(
                error = ?e,
                "processing of inbound gossip failed"
            );
        });
}

pub async fn process_fragments<S>(
    stream: S,
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    span: Span,
) where
    S: TryStream<Ok = net_data::Fragment, Error = Error>,
{
    let sink = FragmentProcessor::new(mbox, node_id, global_state, span);
    stream
        .into_stream()
        .forward(sink)
        .await
        .unwrap_or_else(|e| {
            tracing::debug!(error = ?e, "processing of inbound subscription stream failed");
        });
}

#[must_use = "sinks do nothing unless polled"]
pub struct BlockAnnouncementProcessor {
    mbox: MessageBox<BlockMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    pending_processing: PendingProcessing,
    span: Span,
}

impl BlockAnnouncementProcessor {
    pub(super) fn new(
        mbox: MessageBox<BlockMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        span: Span,
    ) -> Self {
        BlockAnnouncementProcessor {
            mbox,
            node_id,
            global_state,
            pending_processing: PendingProcessing::default(),
            span,
        }
    }

    pub fn message_box(&self) -> MessageBox<BlockMsg> {
        self.mbox.clone()
    }

    fn refresh_stat(&mut self) {
        let state = self.global_state.clone();
        let node_id = self.node_id;
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_block(node_id).await;
            if !refreshed {
                tracing::debug!("received block from node that is not in the peer map");
            }
        }
        .instrument(self.span.clone());
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.pending_processing.start(fut);
    }

    fn poll_flush_mbox(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Pin::new(&mut self.mbox)
            .poll_flush(cx)
            .map_err(handle_mbox_error)
    }
}

#[must_use = "sinks do nothing unless polled"]
pub struct FragmentProcessor {
    mbox: MessageBox<TransactionMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    buffered_fragments: Vec<Fragment>,
    pending_processing: PendingProcessing,
    span: Span,
}

impl FragmentProcessor {
    pub(super) fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        span: Span,
    ) -> Self {
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            buffered_fragments: Vec::with_capacity(buffer_sizes::inbound::FRAGMENTS),
            pending_processing: PendingProcessing::default(),
            span,
        }
    }

    fn refresh_stat(&mut self) {
        let refresh_span = self.span.clone();
        let state = self.global_state.clone();
        let node_id = self.node_id;
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_fragment(node_id).await;
            if !refreshed {
                tracing::debug!("received fragment from node that is not in the peer map",);
            }
        }
        .instrument(refresh_span);
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.pending_processing.start(fut);
    }
}

pub struct GossipProcessor {
    mbox: MessageBox<TopologyMsg>,
    node_id: Address,
    global_state: GlobalStateR,
    span: Span,
    pending_processing: PendingProcessing,
}

impl GossipProcessor {
    pub(super) fn new(
        mbox: MessageBox<TopologyMsg>,
        node_id: Address,
        global_state: GlobalStateR,
        span: Span,
    ) -> Self {
        GossipProcessor {
            mbox,
            node_id,
            global_state,
            span,
            pending_processing: Default::default(),
        }
    }
}

impl Sink<net_data::Header> for BlockAnnouncementProcessor {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                ready!(self.as_mut().poll_flush_mbox(cx))?;
                Poll::Pending
            }
            Poll::Ready(()) => self.mbox.poll_ready(cx).map_err(handle_mbox_error),
        }
    }

    fn start_send(mut self: Pin<&mut Self>, raw_header: net_data::Header) -> Result<(), Error> {
        let header = raw_header.decode().map_err(|e| {
            tracing::info!(
                reason = %e.source().unwrap(),
                "failed to decode incoming block announcement header"
            );
            e
        })?;
        let node_id = self.node_id;
        self.mbox
            .start_send(BlockMsg::AnnouncedBlock(header, node_id))
            .map_err(handle_mbox_error)?;
        self.refresh_stat();
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                ready!(self.as_mut().poll_flush_mbox(cx))?;
                Poll::Pending
            }
            Poll::Ready(()) => self.as_mut().poll_flush_mbox(cx),
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        match self.pending_processing.poll_complete(cx) {
            Poll::Pending => {
                ready!(self.as_mut().poll_flush_mbox(cx))?;
                Poll::Pending
            }
            Poll::Ready(()) => Pin::new(&mut self.mbox).poll_close(cx).map_err(|e| {
                tracing::warn!(
                    reason = %e,
                    "failed to close communication channel to the block task"
                );
                Error::new(Code::Internal, e)
            }),
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
        let fragment = raw_fragment.decode().map_err(|e| {
            tracing::info!(
                reason = %e.source().unwrap(),
                "failed to decode incoming fragment"
            );
            e
        })?;
        tracing::debug!(hash = %fragment.hash(), "received fragment");
        self.buffered_fragments.push(fragment);
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if self.buffered_fragments.is_empty() {
                match self.poll_complete_refresh_stat(cx) {
                    Poll::Pending => {
                        ready!(self.poll_flush_mbox(cx))?;
                        return Poll::Pending;
                    }
                    Poll::Ready(()) => return self.poll_flush_mbox(cx),
                }
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
                    tracing::warn!(
                        reason = %e,
                        "failed to close communication channel to the fragment task"
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
        let span = self.span.clone();
        let _enter = span.enter();
        ready!(self.mbox.poll_ready(cx)).map_err(|e| {
            tracing::debug!(reason = %e, "error sending fragments for processing");
            Error::new(Code::Internal, e)
        })?;
        let fragments = mem::replace(
            &mut self.buffered_fragments,
            Vec::with_capacity(buffer_sizes::inbound::FRAGMENTS),
        );
        self.mbox
            .start_send(TransactionMsg::SendTransaction(
                FragmentOrigin::Network,
                fragments,
            ))
            .map_err(|e| {
                tracing::error!(
                    reason = %e,
                    "failed to send fragments to the fragment task"
                );
                Error::new(Code::Internal, e)
            })?;
        self.refresh_stat();
        Poll::Ready(Ok(()))
    }

    fn poll_flush_mbox(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        let _enter = self.span.enter();
        Pin::new(&mut self.mbox).poll_flush(cx).map_err(|e| {
            tracing::error!(
                reason = %e,
                "communication channel to the fragment task failed"
            );
            Error::new(Code::Internal, e)
        })
    }

    fn poll_complete_refresh_stat(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let _enter = self.span.enter();
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
        let span = self.span.clone();
        let _enter = span.enter();
        let nodes = gossip.nodes.decode().map_err(|e| {
            tracing::info!(
                reason = %e.source().unwrap(),
                "failed to decode incoming gossip"
            );
            e
        })?;
        tracing::debug!("received gossip on {} nodes", nodes.len());
        let (nodes, filtered_out): (Vec<_>, Vec<_>) = nodes
            .into_iter()
            .partition(|node| filter_gossip_node(node, &self.global_state.config));
        if !filtered_out.is_empty() {
            tracing::debug!("nodes dropped from gossip: {:?}", filtered_out);
        }
        let state1 = self.global_state.clone();
        let mut mbox = self.mbox.clone();
        let node_id = self.node_id;
        let fut = future::join(
            async move {
                let refreshed = state1.peers.refresh_peer_on_gossip(node_id).await;
                if !refreshed {
                    tracing::debug!("received gossip from node that is not in the peer map",);
                }
            },
            async move {
                mbox.send(TopologyMsg::AcceptGossip(nodes.into()))
                    .await
                    .unwrap_or_else(|err| {
                        tracing::error!("cannot send gossips to topology: {}", err)
                    });
            },
        )
        .instrument(span.clone())
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
