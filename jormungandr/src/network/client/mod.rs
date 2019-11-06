mod connect;

use super::{
    buffer_sizes,
    inbound::InboundProcessing,
    p2p::{
        comm::{OutboundSubscription, PeerComms},
        Gossip as NodeData, Id,
    },
    subscription::{BlockAnnouncementProcessor, FragmentProcessor, GossipProcessor},
    Channels, GlobalStateR,
};
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
    utils::task::TaskMessageBox,
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
    S: BlockService + FragmentService + GossipService,
{
    service: S,
    logger: Logger,
    global_state: GlobalStateR,
    inbound: InboundSubscriptions<S>,
    block_solicitations: OutboundSubscription<Vec<HeaderHash>>,
    chain_pulls: OutboundSubscription<ChainPullRequest<HeaderHash>>,
    block_sink: BlockAnnouncementProcessor,
    fragment_sink: FragmentProcessor,
    gossip_processor: GossipProcessor,
    incoming_block_announcement: Option<Header>,
    incoming_fragment: Option<Fragment>,
    // FIXME: kill it with fire
    client_box: TaskMessageBox<ClientMsg>,
}

struct ClientBuilder {
    pub logger: Logger,
    pub channels: Channels,
}

impl<S> Client<S>
where
    S: BlockService + FragmentService + GossipService,
{
    pub fn remote_node_id(&self) -> Id {
        self.inbound.node_id
    }

    pub fn logger(&self) -> &Logger {
        &self.logger
    }
}

impl<S> Client<S>
where
    S: core_client::Client,
    S: P2pService<NodeId = Id>,
    S: BlockService<Block = Block>,
    S: FragmentService<Fragment = Fragment>,
    S: GossipService<Node = NodeData>,
{
    fn new(
        inner: S,
        builder: ClientBuilder,
        global_state: GlobalStateR,
        inbound: InboundSubscriptions<S>,
        comms: &mut PeerComms,
    ) -> Self {
        let remote_node_id = inbound.node_id;
        let logger = builder
            .logger
            .new(o!("node_id" => remote_node_id.to_string()));

        let block_sink = BlockAnnouncementProcessor::new(
            builder.channels.block_box,
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "block_events", "direction" => "in")),
        );
        let fragment_sink = FragmentProcessor::new(
            builder.channels.transaction_box,
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "fragments", "direction" => "in")),
        );
        let gossip_processor = GossipProcessor::new(
            remote_node_id,
            global_state.clone(),
            logger.new(o!("stream" => "gossip", "direction" => "in")),
        );

        Client {
            service: inner,
            logger,
            global_state,
            inbound,
            block_solicitations: comms.subscribe_to_block_solicitations(),
            chain_pulls: comms.subscribe_to_chain_pulls(),
            block_sink,
            fragment_sink,
            gossip_processor,
            client_box: builder.channels.client_box,
            incoming_block_announcement: None,
            incoming_fragment: None,
        }
    }
}

struct InboundSubscriptions<S>
where
    S: BlockService + FragmentService + GossipService,
{
    pub node_id: Id,
    pub block_events: <S as BlockService>::BlockSubscription,
    pub fragments: <S as FragmentService>::FragmentSubscription,
    pub gossip: <S as GossipService>::GossipSubscription,
}

#[derive(Copy, Clone)]
enum ProcessingOutcome {
    Continue,
    Disconnect,
}

struct Progress(pub Option<ProcessingOutcome>);

impl Progress {
    fn update(&mut self, async_outcome: Async<ProcessingOutcome>) {
        use self::ProcessingOutcome::*;
        if let Async::Ready(outcome) = async_outcome {
            match (self.0, outcome) {
                (None, outcome) | (Some(Continue), outcome) => {
                    self.0 = Some(outcome);
                }
                (Some(Disconnect), _) => {}
            }
        }
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S: FragmentService + GossipService,
    S::PushHeadersFuture: Send + 'static,
    S::UploadBlocksFuture: Send + 'static,
{
    fn process_block_event(&mut self) -> Poll<ProcessingOutcome, ()> {
        use self::ProcessingOutcome::*;

        // Drive sending of a message to block task to completion
        // before polling more events from the block subscription
        // stream.
        if let Some(header) = self.incoming_block_announcement.take() {
            match self.block_sink.start_send(header).map_err(|_| ())? {
                AsyncSink::Ready => {}
                AsyncSink::NotReady(header) => {
                    self.incoming_block_announcement = Some(header);
                    return Ok(Async::NotReady);
                }
            }
        } else {
            // Ignoring possible NotReady return here: due to the following
            // try_ready!() invocation, this function cannot return Continue
            // while no progress has been made.
            self.block_sink.poll_complete().map_err(|_| ())?;
        }
        let maybe_event = try_ready!(self.inbound.block_events.poll().map_err(|e| {
            debug!(
                self.logger,
                "block subscription stream failure";
                "error" => ?e,
            );
        }));
        let event = match maybe_event {
            Some(event) => event,
            None => {
                debug!(self.logger, "block event subscription ended by the peer");
                return Ok(Disconnect.into());
            }
        };
        trace!(
            self.logger,
            "received block event";
            "stream" => "block_events",
            "direction" => "in",
            "item" => ?event,
        );
        match event {
            BlockEvent::Announce(header) => {
                info!(self.logger, "received block announcement"; "hash" => %header.hash());
                debug_assert!(self.incoming_block_announcement.is_none());
                self.incoming_block_announcement = Some(header);
            }
            BlockEvent::Solicit(block_ids) => {
                debug!(self.logger, "peer requests {} blocks", block_ids.len());
                let (reply_handle, stream) = intercom::stream_reply::<
                    Block,
                    network_core::error::Error,
                >(self.logger.clone());
                self.client_box
                    .send_to(ClientMsg::GetBlocks(block_ids, reply_handle));
                let done_logger = self.logger.clone();
                let err_logger = self.logger.clone();
                self.global_state.spawn(
                    self.service
                        .upload_blocks(stream)
                        .map(move |_| {
                            debug!(done_logger, "finished uploading blocks");
                        })
                        .map_err(move |e| {
                            info!(
                                err_logger,
                                "UploadBlocks request failed";
                                "error" => ?e,
                            );
                        }),
                );
            }
            BlockEvent::Missing(req) => {
                debug!(
                    self.logger,
                    "peer requests missing part of the chain";
                    "checkpoints" => ?req.from,
                    "to" => ?req.to);
                self.push_missing_headers(req);
            }
        }
        Ok(Continue.into())
    }

    fn push_missing_headers(&mut self, req: ChainPullRequest<HeaderHash>) {
        let (reply_handle, stream) =
            intercom::stream_reply::<Header, network_core::error::Error>(self.logger.clone());
        self.client_box
            .send_to(ClientMsg::GetHeadersRange(req.from, req.to, reply_handle));
        let done_logger = self.logger.clone();
        let err_logger = self.logger.clone();
        self.global_state.spawn(
            self.service
                .push_headers(stream)
                .map(move |_| {
                    debug!(done_logger, "finished pushing headers");
                })
                .map_err(move |e| {
                    info!(
                        err_logger,
                        "PushHeaders request failed";
                        "error" => ?e,
                    );
                }),
        );
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S: FragmentService + GossipService,
    S::PullHeadersFuture: Send + 'static,
    S::PullHeadersStream: Send + 'static,
{
    fn pull_headers(&mut self, req: ChainPullRequest<HeaderHash>) {
        let block_box = self.block_sink.message_box();
        let logger = self.logger.new(o!("request" => "PullHeaders"));
        let req_err_logger = logger.clone();
        let res_logger = logger.clone();
        let (handle, sink) = intercom::stream_request::<Header, (), core_error::Error>(
            buffer_sizes::CHAIN_PULL,
            logger.clone(),
        );
        // TODO: make sure that back pressure on the number of requests
        // in flight, imposed through self.service.poll_ready(),
        // prevents unlimited spawning of these tasks.
        // https://github.com/input-output-hk/jormungandr/issues/1034
        self.global_state.spawn(
            block_box
                .send(BlockMsg::ChainHeaders(handle))
                .map_err(move |e| {
                    error!(
                        logger,
                        "failed to enqueue request for processing";
                        "reason" => %e,
                    );
                })
                .map(|_mbox| ()),
        );
        self.global_state.spawn(
            self.service
                .pull_headers(&req.from, &req.to)
                .map_err(move |e| {
                    info!(
                        req_err_logger,
                        "request failed";
                        "reason" => %e,
                    );
                })
                .and_then(move |stream| {
                    sink.send_all(stream)
                        .map_err(move |e| {
                            info!(
                                res_logger,
                                "response stream failed";
                                "reason" => %e,
                            );
                        })
                        .map(|_| ())
                }),
        );
    }
}

impl<S> Client<S>
where
    S: BlockService<Block = Block>,
    S: FragmentService + GossipService,
    S::PullBlocksToTipFuture: Send + 'static,
    S::PullBlocksStream: Send + 'static,
{
    // Temporary support for pulling chain blocks without two-stage
    // retrieval.
    fn pull_blocks_to_tip(&mut self, req: ChainPullRequest<HeaderHash>) {
        let block_box = self.block_sink.message_box();
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
    S: FragmentService + GossipService,
    S::GetBlocksFuture: Send + 'static,
    S::GetBlocksStream: Send + 'static,
{
    fn solicit_blocks(&mut self, block_ids: &[HeaderHash]) {
        let block_box = self.block_sink.message_box();
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

impl<S> Client<S>
where
    S: FragmentService<Fragment = Fragment>,
    S: BlockService + GossipService,
{
    fn process_fragments(&mut self) -> Poll<ProcessingOutcome, ()> {
        use self::ProcessingOutcome::*;

        // Drive sending of a message to fragment task to completion
        // before polling more events from the fragment subscription
        // stream.
        if let Some(fragment) = self.incoming_fragment.take() {
            match self.fragment_sink.start_send(fragment).map_err(|_| ())? {
                AsyncSink::Ready => {}
                AsyncSink::NotReady(fragment) => {
                    self.incoming_fragment = Some(fragment);
                    return Ok(Async::NotReady);
                }
            }
        } else {
            // Ignoring possible NotReady return here: due to the following
            // try_ready!() invocation, this function cannot return Continue
            // while no progress has been made.
            self.fragment_sink.poll_complete().map_err(|_| ())?;
        }

        let maybe_fragment = try_ready!(self.inbound.fragments.poll().map_err(|e| {
            debug!(
                self.logger,
                "fragment stream failure";
                "error" => %e,
            );
        }));
        match maybe_fragment {
            Some(fragment) => {
                trace!(
                    self.logger,
                    "received fragment";
                    "stream" => "fragments",
                    "direction" => "in",
                    "item" => ?fragment,
                );
                debug_assert!(self.incoming_fragment.is_none());
                self.incoming_fragment = Some(fragment);
                Ok(Continue.into())
            }
            None => {
                debug!(self.logger, "fragment subscription ended by the peer");
                Ok(Disconnect.into())
            }
        }
    }
}

impl<S> Client<S>
where
    S: P2pService<NodeId = Id>,
    S: GossipService<Node = NodeData>,
    S: BlockService + FragmentService,
{
    fn process_gossip(&mut self) -> Poll<ProcessingOutcome, ()> {
        use self::ProcessingOutcome::*;

        let maybe_gossip = try_ready!(self.inbound.gossip.poll().map_err(|e| {
            debug!(
                self.logger,
                "gossip stream failure";
                "error" => %e,
            );
        }));
        match maybe_gossip {
            Some(gossip) => {
                trace!(
                    self.logger,
                    "received gossip";
                    "stream" => "gossip",
                    "direction" => "in",
                    "item" => ?gossip,
                );
                self.gossip_processor.process_item(gossip);
                Ok(Continue.into())
            }
            None => {
                debug!(self.logger, "gossip subscription ended by the peer");
                Ok(Disconnect.into())
            }
        }
    }
}

impl<S> Future for Client<S>
where
    S: core_client::Client,
    S: P2pService<NodeId = Id>,
    S: BlockService<Block = Block>,
    S: FragmentService<Fragment = Fragment>,
    S: GossipService<Node = NodeData>,
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
        use self::ProcessingOutcome::*;

        loop {
            // Drive any pending activity of the gRPC client until it is ready
            // to process another request.
            try_ready!(self.service.poll_ready().map_err(|e| {
                info!(
                    self.logger,
                    "client connection broke down";
                    "error" => ?e);
            }));

            let mut progress = Progress(None);

            progress.update(self.process_block_event()?);
            progress.update(self.process_fragments()?);
            progress.update(self.process_gossip()?);

            // Block solicitations and chain pulls are special:
            // they are handled with client requests on the client side,
            // but on the server side, they are fed into the block event stream.
            progress.update(self.block_solicitations.poll().unwrap().map(|maybe_item| {
                match maybe_item {
                    Some(block_ids) => {
                        self.solicit_blocks(&block_ids);
                        Continue
                    }
                    None => {
                        debug!(self.logger, "outbound block solicitation stream closed");
                        Disconnect
                    }
                }
            }));
            progress.update(
                self.chain_pulls
                    .poll()
                    .unwrap()
                    .map(|maybe_item| match maybe_item {
                        Some(req) => {
                            self.pull_headers(req);
                            Continue
                        }
                        None => {
                            debug!(self.logger, "outbound header pull stream closed");
                            Disconnect
                        }
                    }),
            );

            match progress {
                Progress(None) => return Ok(Async::NotReady),
                Progress(Some(Continue)) => continue,
                Progress(Some(Disconnect)) => {
                    info!(self.logger, "disconnecting client");
                    return Ok(().into());
                }
            }
        }
    }
}
