use super::{
    chain_pull, grpc,
    inbound::InboundProcessing,
    p2p::comm::{PeerComms, Subscription},
    p2p::topology,
    subscription::{self, SendingBlockMsg},
    Channels, ConnectionState, GlobalStateR,
};
use crate::{
    blockcfg::{Block, Fragment, Header, HeaderHash},
    intercom::{self, BlockMsg, ClientMsg},
};
use futures::prelude::*;
use network_core::client::{self as core_client, Client as _};
use network_core::client::{BlockService, FragmentService, GossipService, P2pService};
use network_core::error as core_error;
use network_core::gossip::{Gossip, Node};
use network_core::subscription::{BlockEvent, ChainPullRequest};
use slog::Logger;

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

struct EarlyOutbound {
    pub block_announcements: Subscription<Header>,
    pub fragments: Subscription<Fragment>,
    pub gossip: Subscription<Gossip<topology::NodeData>>,
    pub block_solicitations: Subscription<Vec<HeaderHash>>,
    pub chain_pulls: Subscription<ChainPullRequest<HeaderHash>>,
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
    fn subscribe(
        service: S,
        state: ConnectionState,
        outbound: EarlyOutbound,
        channels: Channels,
    ) -> impl Future<Item = Self, Error = ()> {
        let block_announcements = outbound.block_announcements;
        let fragments = outbound.fragments;
        let gossip = outbound.gossip;
        let block_solicitations = outbound.block_solicitations;
        let chain_pulls = outbound.chain_pulls;
        let err_logger = state.logger().clone();
        service
            .ready()
            .and_then(move |mut service| {
                let block_req = service.block_subscription(block_announcements);
                service.ready().map(move |service| (service, block_req))
            })
            .and_then(move |(mut service, block_req)| {
                let content_req = service.fragment_subscription(fragments);
                service
                    .ready()
                    .map(move |service| (service, block_req, content_req))
            })
            .and_then(move |(mut service, block_req, content_req)| {
                let gossip_req = service.gossip_subscription(gossip);
                block_req.join3(content_req, gossip_req).map(
                    move |(block_res, content_res, gossip_res)| {
                        (service, block_res, content_res, gossip_res)
                    },
                )
            })
            .map_err(move |err| {
                info!(err_logger, "subscription request failed: {:?}", err);
            })
            .and_then(
                move |(
                    service,
                    (block_events, node_id),
                    (fragment_sub, node_id_1),
                    (gossip_sub, node_id_2),
                )| {
                    if node_id != node_id_1 {
                        warn!(
                            state.logger(),
                            "peer subscription IDs do not match: {} != {}", node_id, node_id_1
                        );
                        return Err(());
                    }
                    if node_id != node_id_2 {
                        warn!(
                            state.logger(),
                            "peer subscription IDs do not match: {} != {}", node_id, node_id_2
                        );
                        return Err(());
                    }
                    let logger = state.logger().new(o!("node_id" => node_id.to_string()));

                    // Spin off processing tasks for subscriptions that can be
                    // managed with just the global state.
                    subscription::process_fragments(
                        fragment_sub,
                        node_id,
                        state.global.clone(),
                        channels.transaction_box.clone(),
                        logger.clone(),
                    );
                    subscription::process_gossip(
                        gossip_sub,
                        node_id,
                        state.global.clone(),
                        logger.clone(),
                    );

                    // Resolve with the client instance.
                    let client = Client {
                        service,
                        logger,
                        global_state: state.global,
                        channels,
                        remote_node_id: node_id,
                        block_events,
                        block_solicitations,
                        chain_pulls,
                        sending_block_msg: None,
                    };
                    Ok(client)
                },
            )
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

pub fn connect(
    state: ConnectionState,
    channels: Channels,
) -> (
    PeerComms,
    impl Future<Item = Client<grpc::Connection>, Error = ()>,
) {
    let addr = state.connection;
    let expected_block0 = state.global.block0_hash;
    let connect_err_logger = state.logger().clone();
    let ready_err_logger = state.logger().clone();
    let handshake_err_logger = state.logger().clone();
    let block0_mismatch_logger = state.logger().clone();
    let mut peer_comms = PeerComms::new();
    let outbound = EarlyOutbound {
        block_announcements: peer_comms.subscribe_to_block_announcements(),
        fragments: peer_comms.subscribe_to_fragments(),
        gossip: peer_comms.subscribe_to_gossip(),
        block_solicitations: peer_comms.subscribe_to_block_solicitations(),
        chain_pulls: peer_comms.subscribe_to_chain_pulls(),
    };

    let future = grpc::connect(addr, Some(state.global.as_ref().topology.node().id()))
        .map_err(move |e| {
            if let Some(e) = e.connect_error() {
                info!(connect_err_logger, "error connecting to peer"; "reason" => %e);
            } else if let Some(e) = e.http_error() {
                info!(connect_err_logger, "HTTP/2 handshake error"; "reason" => %e);
            } else {
                warn!(connect_err_logger, "error while connecting to peer"; "error" => ?e);
            }
        })
        .and_then(move |conn| {
            conn.ready().map_err(move |e| {
                warn!(
                    ready_err_logger,
                    "gRPC client error after connecting: {:?}", e
                );
            })
        })
        .and_then(move |mut conn| {
            conn.handshake()
                .map_err(move |e| {
                    info!(handshake_err_logger, "protocol handshake failed: {:?}", e);
                })
                .and_then(move |block0| {
                    if block0 == expected_block0 {
                        Ok(conn)
                    } else {
                        warn!(
                            block0_mismatch_logger,
                            "block 0 hash {} in handshake is not expected {}",
                            block0,
                            expected_block0
                        );
                        Err(())
                    }
                })
        })
        .and_then(move |conn| Client::subscribe(conn, state, outbound, channels))
        .inspect(|client| {
            debug!(client.logger(), "connected to peer");
        });

    (peer_comms, future)
}
