mod connect;

use super::{
    chain_pull,
    inbound::InboundProcessing,
    p2p::comm::{PeerComms, Subscription},
    p2p::topology,
    subscription::{self, SendingBlockMsg},
    Channels, GlobalStateR,
};
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
};
use network_core::client as core_client;
use network_core::client::{BlockService, FragmentService, GossipService, P2pService};
use network_core::error as core_error;
use network_core::subscription::{BlockEvent, ChainPullRequest};

use futures::prelude::*;
use slog::Logger;

pub use self::connect::{connect, ConnectError, ConnectFuture, ConnectHandle};

#[must_use = "Client must be polled"]
pub struct Client<S>
where
    S: BlockService,
{
    service: S,
    logger: Logger,
    global_state: GlobalStateR,
    channels: Channels,
    remote_node_id: topology::NodeId,
    block_events: S::BlockSubscription,
    block_solicitations: Subscription<Vec<HeaderHash>>,
    chain_pulls: Subscription<ChainPullRequest<HeaderHash>>,
    sending_block_msg: Option<SendingBlockMsg>,
}

struct ClientBuilder {
    pub logger: Logger,
    pub channels: Channels,
}

impl<S: BlockService> Client<S> {
    pub fn remote_node_id(&self) -> topology::NodeId {
        self.remote_node_id
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl<S> Client<S>
where
    S: core_client::Client,
    S: P2pService<NodeId = topology::NodeId>,
    S: BlockService<Block = Block>,
    S: FragmentService<Fragment = Fragment>,
    S: GossipService<Node = topology::NodeData>,
    S::UploadBlocksFuture: Send + 'static,
    S::FragmentSubscription: Send + 'static,
    S::GossipSubscription: Send + 'static,
{
    fn new(
        inner: S,
        builder: ClientBuilder,
        global_state: GlobalStateR,
        inbound: connect::InboundSubscriptions<S>,
        comms: &mut PeerComms,
    ) -> Self {
        let remote_node_id = inbound.node_id;
        let logger = builder
            .logger
            .new(o!("node_id" => remote_node_id.to_string()));

        // Spin off processing tasks for subscriptions that can be
        // managed with just the global state.
        subscription::process_fragments(
            inbound.fragments,
            remote_node_id,
            global_state.clone(),
            builder.channels.transaction_box.clone(),
            logger.clone(),
        );
        subscription::process_gossip(
            inbound.gossip,
            remote_node_id,
            global_state.clone(),
            logger.clone(),
        );

        Client {
            service: inner,
            logger,
            global_state,
            channels: builder.channels,
            remote_node_id,
            block_events: inbound.block_events,
            block_solicitations: comms.subscribe_to_block_solicitations(),
            chain_pulls: comms.subscribe_to_chain_pulls(),
            sending_block_msg: None,
        }
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::PushHeadersFuture: Send + 'static,
    S::UploadBlocksFuture: Send + 'static,
{
    fn process_block_event(&mut self, event: BlockEvent<S::Block>) {
        match event {
            BlockEvent::Announce(header) => {
                debug!(self.logger, "received block event Announce");
                let future = subscription::process_block_announcement(
                    header,
                    self.remote_node_id,
                    &self.global_state,
                    self.channels.block_box.clone(),
                );
                self.sending_block_msg = Some(future);
            }
            BlockEvent::Solicit(block_ids) => {
                debug!(self.logger, "received block event Solicit");
                let (reply_handle, stream) = intercom::stream_reply::<
                    Block,
                    network_core::error::Error,
                >(self.logger.clone());
                self.channels
                    .client_box
                    .send_to(ClientMsg::GetBlocks(block_ids, reply_handle));
                let node_id = self.remote_node_id;
                let done_logger = self.logger.clone();
                let err_logger = self.logger.clone();
                self.global_state.spawn(
                    self.service
                        .upload_blocks(stream)
                        .map(move |_| {
                            debug!(done_logger, "finished uploading blocks to {}", node_id);
                        })
                        .map_err(move |err| {
                            info!(err_logger, "UploadBlocks request failed: {:?}", err);
                        }),
                );
            }
            BlockEvent::Missing(req) => {
                debug!(self.logger, "received block event Missing");
                self.push_missing_blocks(req);
            }
        }
    }

    // FIXME: use this to handle BlockEvent::Missing events when two-stage
    // chain pull processing is implemented in the blockchain task.
    #[allow(dead_code)]
    fn push_missing_headers(&mut self, req: ChainPullRequest<HeaderHash>) {
        let (reply_handle, stream) =
            intercom::stream_reply::<Header, network_core::error::Error>(self.logger.clone());
        self.channels.client_box.send_to(ClientMsg::GetHeadersRange(
            req.from,
            req.to,
            reply_handle,
        ));
        let node_id = self.remote_node_id;
        let done_logger = self.logger.clone();
        let err_logger = self.logger.clone();
        self.global_state.spawn(
            self.service
                .push_headers(stream)
                .map(move |_| {
                    debug!(done_logger, "finished pushing headers to {}", node_id);
                })
                .map_err(move |err| {
                    info!(err_logger, "PushHeaders request failed: {:?}", err);
                }),
        );
    }

    // Temporary support for pushing chain blocks without two-stage
    // retrieval.
    fn push_missing_blocks(&mut self, req: ChainPullRequest<HeaderHash>) {
        let (reply_handle, stream) =
            intercom::stream_reply::<Block, network_core::error::Error>(self.logger.clone());
        self.channels
            .client_box
            .send_to(ClientMsg::PullBlocksToTip(req.from, reply_handle));
        let node_id = self.remote_node_id;
        let done_logger = self.logger.clone();
        let err_logger = self.logger.clone();
        self.global_state.spawn(
            self.service
                .upload_blocks(stream)
                .map(move |_| {
                    debug!(done_logger, "finished pushing blocks to {}", node_id);
                })
                .map_err(move |err| {
                    info!(err_logger, "UploadBlocks request failed: {:?}", err);
                }),
        );
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::PullHeadersFuture: Send + 'static,
    S::PullHeadersStream: Send + 'static,
{
    // FIXME: use this to handle chain pull requests when two-stage
    // chain pull processing is implemented in the blockchain task.
    #[allow(dead_code)]
    fn pull_headers(&mut self, req: ChainPullRequest<HeaderHash>) {
        let block_box = self.channels.block_box.clone();
        let logger = self.logger.clone();
        let err_logger = logger.clone();
        self.global_state.spawn(
            self.service
                .pull_headers(&req.from, &req.to)
                .map_err(move |e| {
                    info!(err_logger, "PullHeaders request failed: {:?}", e);
                })
                .and_then(move |stream| {
                    let err2_logger = logger.clone();
                    let err3_logger = logger.clone();
                    let (handle, sink) = intercom::stream_request::<Header, core_error::Error>(
                        chain_pull::CHUNK_SIZE,
                    );
                    block_box
                        .send(BlockMsg::ChainHeaders(handle))
                        .map_err(move |e| {
                            error!(err2_logger, "sending to block task failed: {:?}", e);
                        })
                        .and_then(move |_| {
                            sink.send_all(stream).map(|_| {}).map_err(move |e| {
                                warn!(
                                    err3_logger,
                                    "processing of PullHeaders response stream failed: {:?}", e
                                );
                            })
                        })
                }),
        );
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::PullBlocksToTipFuture: Send + 'static,
    S::PullBlocksStream: Send + 'static,
{
    // Temporary support for pulling chain blocks without two-stage
    // retrieval.
    fn pull_blocks_to_tip(&mut self, req: ChainPullRequest<HeaderHash>) {
        let block_box = self.channels.block_box.clone();
        let logger = self.logger.clone();
        let err_logger = logger.clone();
        self.global_state.spawn(
            self.service
                .pull_blocks_to_tip(&req.from)
                .map_err(move |e| {
                    info!(err_logger, "PullBlocksToTip request failed: {:?}", e);
                })
                .and_then(move |stream| {
                    let stream_err_logger = logger.clone();
                    let sink_err_logger = logger.clone();
                    let stream = stream.map_err(move |e| {
                        info!(
                            stream_err_logger,
                            "PullBlocksToTip response stream failed: {:?}", e
                        );
                    });
                    InboundProcessing::with_unary(
                        block_box.clone(),
                        logger.clone(),
                        |block, reply| BlockMsg::NetworkBlock(block, reply),
                    )
                    .sink_map_err(move |e| {
                        warn!(sink_err_logger, "pulled block validation failed: {:?}", e)
                    })
                    .send_all(stream)
                    .map(move |_| {
                        debug!(logger, "PullBlocksToTip response processed");
                    })
                }),
        );
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S::GetBlocksFuture: Send + 'static,
    S::GetBlocksStream: Send + 'static,
{
    fn solicit_blocks(&mut self, block_ids: &[HeaderHash]) {
        let block_box = self.channels.block_box.clone();
        let logger = self.logger.clone();
        let err_logger = logger.clone();
        self.global_state.spawn(
            self.service
                .get_blocks(block_ids)
                .map_err(move |e| {
                    info!(
                        err_logger,
                        "GetBlocks request (solicitation) failed: {:?}", e
                    );
                })
                .and_then(move |stream| {
                    let stream_err_logger = logger.clone();
                    let sink_err_logger = logger.clone();
                    let stream = stream.map_err(move |e| {
                        info!(
                            stream_err_logger,
                            "GetBlocks response stream failed: {:?}", e
                        );
                    });
                    InboundProcessing::with_unary(
                        block_box.clone(),
                        logger.clone(),
                        |block, reply| BlockMsg::NetworkBlock(block, reply),
                    )
                    .sink_map_err(move |e| {
                        warn!(sink_err_logger, "network block validation failed: {:?}", e)
                    })
                    .send_all(stream)
                    .map(move |_| {
                        debug!(logger, "GetBlocks response processed");
                    })
                }),
        );
    }
}

impl<S> Future for Client<S>
where
    S: core_client::Client,
    S: BlockService<Block = Block>,
    S::GetBlocksFuture: Send + 'static,
    S::GetBlocksStream: Send + 'static,
    S::PullBlocksToTipFuture: Send + 'static,
    S::PullBlocksStream: Send + 'static,
    S::PullHeadersFuture: Send + 'static,
    S::PullHeadersStream: Send + 'static,
    S::PushHeadersFuture: Send + 'static,
    S::UploadBlocksFuture: Send + 'static,
{
    type Item = ();
    type Error = ();
    fn poll(&mut self) -> Poll<(), ()> {
        loop {
            // Drive any pending activity of the gRPC client until it is ready
            // to process another request.
            try_ready!(self.service.poll_ready().map_err(|e| {
                info!(self.logger, "P2P client connection error: {:?}", e);
            }));
            let mut streams_ready = false;
            if let Some(ref mut future) = self.sending_block_msg {
                // Drive sending of a message to block task to completion
                // before polling more events from the block subscription
                // stream.
                let send_polled = future.poll().map_err(|e| {
                    error!(
                        self.logger,
                        "failed to send message to the block task";
                        "reason" => %e
                    );
                })?;
                match send_polled {
                    Async::NotReady => {}
                    Async::Ready(_) => {
                        self.sending_block_msg = None;
                    }
                }
            } else {
                let block_event_polled = self.block_events.poll().map_err(|e| {
                    debug!(self.logger, "block subscription stream failure: {:?}", e);
                })?;
                match block_event_polled {
                    Async::NotReady => {}
                    Async::Ready(None) => {
                        debug!(self.logger, "block subscription stream terminated");
                        return Ok(().into());
                    }
                    Async::Ready(Some(event)) => {
                        streams_ready = true;
                        self.process_block_event(event);
                    }
                }
            }
            // Block solicitations and chain pulls are special:
            // they are handled with client requests on the client side,
            // but on the server side, they are fed into the block event stream.
            match self.block_solicitations.poll().unwrap() {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!(self.logger, "outbound block solicitation stream closed");
                    return Ok(().into());
                }
                Async::Ready(Some(block_ids)) => {
                    streams_ready = true;
                    self.solicit_blocks(&block_ids);
                }
            }
            match self.chain_pulls.poll().unwrap() {
                Async::NotReady => {}
                Async::Ready(None) => {
                    debug!(self.logger, "outbound header pull stream closed");
                    return Ok(().into());
                }
                Async::Ready(Some(req)) => {
                    streams_ready = true;
                    // FIXME: implement two-stage chain pull processing
                    // in the blockchain task and use pull_headers here.
                    self.pull_blocks_to_tip(req);
                }
            }
            if !streams_ready {
                return Ok(Async::NotReady);
            }
        }
    }
}
