use super::{buffer_sizes, convert::Decode, GlobalStateR};
use crate::{
    blockcfg::Fragment,
    intercom::{self, BlockMsg, TopologyMsg, TransactionMsg},
    settings::start::network::Configuration,
    topology::{Gossip, NodeId},
    utils::async_msg::{self, MessageBox},
};
use chain_network::{
    data as net_data,
    error::{Code, Error},
};
use futures::{future::BoxFuture, prelude::*, ready};
use jormungandr_lib::interfaces::FragmentOrigin;
use std::{
    error::Error as _,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
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
    node_id: NodeId,
    global_state: GlobalStateR,
) where
    S: TryStream<Ok = net_data::Header, Error = Error>,
{
    let sink = BlockAnnouncementProcessor::new(mbox, node_id, global_state);
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
    node_id: NodeId,
    global_state: GlobalStateR,
) where
    S: TryStream<Ok = net_data::Gossip, Error = Error>,
{
    let processor = GossipProcessor::new(mbox, node_id, global_state, Direction::Server);
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
    node_id: NodeId,
    global_state: GlobalStateR,
) where
    S: TryStream<Ok = net_data::Fragment, Error = Error>,
{
    let sink = FragmentProcessor::new(mbox, node_id, global_state);
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
    node_id: NodeId,
    global_state: GlobalStateR,
    pending_processing: PendingProcessing,
}

impl BlockAnnouncementProcessor {
    pub(super) fn new(
        mbox: MessageBox<BlockMsg>,
        node_id: NodeId,
        global_state: GlobalStateR,
    ) -> Self {
        BlockAnnouncementProcessor {
            mbox,
            node_id,
            global_state,
            pending_processing: PendingProcessing::default(),
        }
    }

    pub fn message_box(&self) -> MessageBox<BlockMsg> {
        self.mbox.clone()
    }

    fn refresh_stat(&mut self) {
        let state = self.global_state.clone();
        let node_id = self.node_id;
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_block(&node_id).await;
            if !refreshed {
                tracing::debug!("received block from node that is not in the peer map");
            }
        }
        .in_current_span();
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
    node_id: NodeId,
    global_state: GlobalStateR,
    buffered_fragments: Vec<Fragment>,
    pending_processing: PendingProcessing,
}

impl FragmentProcessor {
    pub(super) fn new(
        mbox: MessageBox<TransactionMsg>,
        node_id: NodeId,
        global_state: GlobalStateR,
    ) -> Self {
        FragmentProcessor {
            mbox,
            node_id,
            global_state,
            buffered_fragments: Vec::with_capacity(buffer_sizes::inbound::FRAGMENTS),
            pending_processing: PendingProcessing::default(),
        }
    }

    fn refresh_stat(&mut self) {
        let state = self.global_state.clone();
        let node_id = self.node_id;
        let fut = async move {
            let refreshed = state.peers.refresh_peer_on_fragment(&node_id).await;
            if !refreshed {
                tracing::debug!("received fragment from node that is not in the peer map",);
            }
        }
        .in_current_span();
        // It's OK to overwrite a pending future because only the latest
        // timestamp matters.
        self.pending_processing.start(fut);
    }
}

pub enum Direction {
    Server,
    Client,
}

pub struct GossipProcessor {
    mbox: MessageBox<TopologyMsg>,
    node_id: NodeId,
    global_state: GlobalStateR,
    pending_processing: PendingProcessing,
    // To keep a healthy pool of p2p peers, we need to keep track of nodes we were able
    // to connect to successfully.
    // However, a server may need to accomodate peers which are not publicy reachable
    // (e.g. private nodes, full wallets, ...) and embedding this process in the handshake
    // procedure is not the best idea.
    // Instead, a peer is "promoted" (i.e. marked as successfully connected in poldercast terminology)
    // after the first gossip is received, which signals interest in participating in the dissemination
    // overlay.
    peer_promoted: bool,
}

impl GossipProcessor {
    pub(super) fn new(
        mbox: MessageBox<TopologyMsg>,
        node_id: NodeId,
        global_state: GlobalStateR,
        direction: Direction,
    ) -> Self {
        GossipProcessor {
            mbox,
            node_id,
            global_state,
            pending_processing: Default::default(),
            // client will handle promotion after handshake since they are connecting to a public
            // node by construction
            peer_promoted: matches!(direction, Direction::Client),
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
            .start_send(BlockMsg::AnnouncedBlock(Box::new(header), node_id))
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
        ready!(self.mbox.poll_ready(cx)).map_err(|e| {
            tracing::debug!(reason = %e, "error sending fragments for processing");
            Error::new(Code::Internal, e)
        })?;
        let fragments = mem::replace(
            &mut self.buffered_fragments,
            Vec::with_capacity(buffer_sizes::inbound::FRAGMENTS),
        );
        let (reply_handle, _reply_future) = intercom::unary_reply();
        self.mbox
            .start_send(TransactionMsg::SendTransactions {
                origin: FragmentOrigin::Network,
                fragments,
                fail_fast: false,
                reply_handle,
            })
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
        Pin::new(&mut self.mbox).poll_flush(cx).map_err(|e| {
            tracing::error!(
                reason = %e,
                "communication channel to the fragment task failed"
            );
            Error::new(Code::Internal, e)
        })
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
        let peer_promoted = std::mem::replace(&mut self.peer_promoted, true);
        let state1 = self.global_state.clone();
        let mut mbox = self.mbox.clone();
        let node_id = self.node_id;
        let fut = future::join(
            async move {
                let refreshed = state1.peers.refresh_peer_on_gossip(&node_id).await;
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
                if !peer_promoted {
                    tracing::info!(%node_id, "promoting peer");
                    mbox.send(TopologyMsg::PromotePeer(node_id))
                        .await
                        .unwrap_or_else(|e| {
                            tracing::error!("Error sending message to topology task: {}", e)
                        });
                }
            },
        )
        .in_current_span()
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
