use super::{
    candidate::{self, CandidateForest},
    chain,
    chain_selection::{self, ComparisonResult},
    Blockchain, Error, ErrorKind, PreCheckedHeader, Ref, Tip, MAIN_BRANCH_TAG,
};
use crate::{
    blockcfg::{Block, FragmentId, Header},
    intercom::{
        self, BlockMsg, ExplorerMsg, NetworkMsg, PropagateMsg, ReplyHandle, TransactionMsg,
    },
    network::p2p::Id as NodeId,
    stats_counter::StatsCounter,
    utils::{
        async_msg::{self, MessageBox, MessageQueue},
        task::TokioServiceInfo,
    },
};
use chain_core::property::{Block as _, Fragment as _, HasHeader as _, Header as _};
use jormungandr_lib::interfaces::FragmentStatus;

use futures::future::{Either, Loop};
use slog::Logger;
use tokio::{
    prelude::*,
    timer::{timeout, Interval, Timeout},
};

use std::{sync::Arc, time::Duration};

type TimeoutError = timeout::Error<Error>;

const DEFAULT_TIMEOUT_PROCESS_LEADERSHIP: u64 = 5;
const DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT: u64 = 5;
const DEFAULT_TIMEOUT_PROCESS_BLOCKS: u64 = 60;
const DEFAULT_TIMEOUT_PROCESS_HEADERS: u64 = 60;

pub struct Process {
    pub blockchain: Blockchain,
    pub blockchain_tip: Tip,
    pub candidate_forest: CandidateForest,
    pub stats_counter: StatsCounter,
    pub network_msgbox: MessageBox<NetworkMsg>,
    pub fragment_msgbox: MessageBox<TransactionMsg>,
    pub explorer_msgbox: Option<MessageBox<ExplorerMsg>>,
    pub garbage_collection_interval: Duration,
}

impl Process {
    pub fn start(
        mut self,
        service_info: TokioServiceInfo,
        input: MessageQueue<BlockMsg>,
    ) -> impl Future<Item = (), Error = ()> {
        service_info.spawn(self.start_garbage_collector(service_info.logger().clone()));
        input.for_each(move |msg| {
            self.handle_input(&service_info, msg);
            future::ok(())
        })
    }

    fn handle_input(&mut self, info: &TokioServiceInfo, input: BlockMsg) {
        let blockchain = self.blockchain.clone();
        let blockchain_tip = self.blockchain_tip.clone();
        let network_msg_box = self.network_msgbox.clone();
        let explorer_msg_box = self.explorer_msgbox.clone();
        let candidate_forest = self.candidate_forest.clone();
        let mut tx_msg_box = self.fragment_msgbox.clone();
        let stats_counter = self.stats_counter.clone();

        match input {
            BlockMsg::LeadershipBlock(block) => {
                let logger = info.logger().new(o!(
                    "hash" => block.header.hash().to_string(),
                    "parent" => block.header.parent_id().to_string(),
                    "date" => block.header.block_date().to_string()));
                let logger2 = logger.clone();
                let logger3 = logger.clone();

                info!(logger, "receiving block from leadership service");

                let process_new_block =
                    process_leadership_block(logger.clone(), blockchain.clone(), block.clone());

                let fragments = block.fragments().map(|f| f.id()).collect();

                let update_mempool = process_new_block.and_then(move |new_block_ref| {
                    debug!(logger2, "updating fragment's log");
                    try_request_fragment_removal(&mut tx_msg_box, fragments, new_block_ref.header())
                        .map_err(|_| "cannot remove fragments from pool".into())
                        .map(|_| new_block_ref)
                });

                let process_new_ref = update_mempool.and_then(move |new_block_ref| {
                    debug!(logger3, "processing the new block and propagating");
                    process_and_propagate_new_ref(
                        logger3,
                        blockchain,
                        blockchain_tip,
                        Arc::clone(&new_block_ref),
                        network_msg_box,
                    )
                });

                let notify_explorer = process_new_ref.and_then(move |()| {
                    if let Some(msg_box) = explorer_msg_box {
                        Either::A(
                            msg_box
                                .send(ExplorerMsg::NewBlock(block))
                                .map_err(|_| "Cannot propagate block to explorer".into())
                                .map(|_| ()),
                        )
                    } else {
                        Either::B(future::ok(()))
                    }
                });

                info.spawn(
                    Timeout::new(notify_explorer, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_LEADERSHIP))
                        .map_err(move |err: TimeoutError| {
                            error!(logger, "cannot process leadership block" ; "reason" => err.to_string())
                        })
                )
            }
            BlockMsg::AnnouncedBlock(header, node_id) => {
                let logger = info.logger().new(o!(
                    "hash" => header.hash().to_string(),
                    "parent" => header.parent_id().to_string(),
                    "date" => header.block_date().to_string(),
                    "from_node_id" => node_id.to_string()));

                info!(logger, "received block announcement from network");

                let future = process_block_announcement(
                    blockchain.clone(),
                    blockchain_tip.clone(),
                    header,
                    node_id,
                    network_msg_box.clone(),
                    logger.clone(),
                );

                info.spawn(Timeout::new(future, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT)).map_err(move |err: TimeoutError| {
                    error!(logger, "cannot process block announcement" ; "reason" => err.to_string())
                }))
            }
            BlockMsg::NetworkBlocks(handle) => {
                struct State<S> {
                    stream: S,
                    reply: ReplyHandle<()>,
                    candidate: Option<Arc<Ref>>,
                }

                info!(info.logger(), "receiving block stream from network");

                let logger = info.logger().clone();
                let logger_fold = logger.clone();
                let logger_err = logger.clone();
                let blockchain_fold = blockchain.clone();
                let (stream, reply) = handle.into_stream_and_reply();
                let stream =
                    stream.map_err(|()| Error::from("Error while processing block input stream"));
                let state = State {
                    stream,
                    reply,
                    candidate: None,
                };
                let future = future::loop_fn(state, move |state| {
                    let blockchain = blockchain_fold.clone();
                    let candidate_forest = candidate_forest.clone();
                    let tx_msg_box = tx_msg_box.clone();
                    let explorer_msg_box = explorer_msg_box.clone();
                    let stats_counter = stats_counter.clone();
                    let logger = logger_fold.clone();
                    let State {
                        stream,
                        reply,
                        candidate,
                    } = state;
                    stream.into_future().map_err(|(e, _)| e).and_then(
                        move |(maybe_block, stream)| match maybe_block {
                            Some(block) => Either::A(
                                process_network_block(
                                    blockchain,
                                    candidate_forest,
                                    block,
                                    tx_msg_box,
                                    explorer_msg_box,
                                    logger.clone(),
                                )
                                .then(move |res| match res {
                                    Ok(candidate) => {
                                        stats_counter.add_block_recv_cnt(1);
                                        Ok(Loop::Continue(State {
                                            stream,
                                            reply,
                                            candidate,
                                        }))
                                    }
                                    Err(e) => {
                                        info!(
                                            logger,
                                            "validation of an incoming block failed";
                                            "reason" => %e,
                                        );
                                        reply.reply_error(network_block_error_into_reply(e));
                                        Ok(Loop::Break(candidate))
                                    }
                                }),
                            ),
                            None => {
                                reply.reply_ok(());
                                Either::B(future::ok(Loop::Break(candidate)))
                            }
                        },
                    )
                })
                .and_then(move |maybe_updated| match maybe_updated {
                    Some(new_block_ref) => {
                        let future = process_and_propagate_new_ref(
                            logger,
                            blockchain,
                            blockchain_tip,
                            Arc::clone(&new_block_ref),
                            network_msg_box,
                        );
                        Either::A(future)
                    }
                    None => Either::B(future::ok(())),
                });

                info.spawn(Timeout::new(future, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_BLOCKS)).map_err(move |err: TimeoutError| {
                    error!(logger_err, "cannot process network blocks" ; "reason" => err.to_string())
                }))
            }
            BlockMsg::ChainHeaders(handle) => {
                info!(info.logger(), "receiving header stream from network");

                let (stream, reply) = handle.into_stream_and_reply();
                let logger = info.logger().clone();
                let logger_err = info.logger().clone();

                let future = candidate_forest.advance_branch(blockchain, stream);
                let future = future.then(move |resp| match resp {
                    Err(e) => {
                        info!(
                            logger,
                            "error processing an incoming header stream";
                            "reason" => %e,
                        );
                        reply.reply_error(chain_header_error_into_reply(e));
                        Either::A(future::ok(()))
                    }
                    Ok((hashes, maybe_remainder)) => {
                        if hashes.is_empty() {
                            Either::A(future::ok(()))
                        } else {
                            Either::B(
                                network_msg_box
                                    .send(NetworkMsg::GetBlocks(hashes))
                                    .map_err(|_| "cannot request blocks from network".into())
                                    .map(|_| reply.reply_ok(())),
                            )
                            // TODO: if the stream is not ended, resume processing
                            // after more blocks arrive
                        }
                    }
                });

                info.spawn(Timeout::new(future, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_HEADERS)).map_err(move |err: TimeoutError| {
                    error!(logger_err, "cannot process network headers" ; "reason" => err.to_string())
                }))
            }
        }
    }

    fn start_garbage_collector(&self, logger: Logger) -> impl Future<Item = (), Error = ()> {
        let candidate_forest = self.candidate_forest.clone();
        let garbage_collection_interval = self.garbage_collection_interval;
        let error_logger = logger.clone();
        Interval::new_interval(garbage_collection_interval)
            .for_each(move |_instant| {
                debug!(logger, "garbage collecting unresolved branch candidates");
                candidate_forest.purge()
            })
            .map_err(move |e| {
                error!(error_logger, "cannot run garbage collection" ; "reason" => %e);
            })
    }
}

fn try_request_fragment_removal(
    tx_msg_box: &mut MessageBox<TransactionMsg>,
    fragment_ids: Vec<FragmentId>,
    header: &Header,
) -> Result<(), async_msg::TrySendError<TransactionMsg>> {
    let hash = header.hash().into();
    let date = header.block_date().clone().into();
    let status = FragmentStatus::InABlock { date, block: hash };
    tx_msg_box.try_send(TransactionMsg::RemoveTransactions(fragment_ids, status))
}

/// process a new candidate block on top of the blockchain, this function may:
///
/// * update the current tip if the candidate's parent is the current tip;
/// * update a branch if the candidate parent is that branch's tip;
/// * create a new branch if none of the above;
///
/// If the current tip is not the one being updated we will then trigger
/// chain selection after updating that other branch as it may be possible that
/// this branch just became more interesting for the current consensus algorithm.
pub fn process_new_ref(
    logger: Logger,
    mut blockchain: Blockchain,
    mut tip: Tip,
    candidate: Arc<Ref>,
) -> impl Future<Item = (), Error = Error> {
    use tokio::prelude::future::Either::*;

    let candidate_hash = candidate.hash();
    let mut storage = blockchain.storage().clone();

    tip.clone()
        .get_ref()
        .and_then(move |tip_ref| {
            if tip_ref.hash() == candidate.block_parent_hash() {
                info!(logger, "update current branch tip");
                A(A(tip.update_ref(candidate).map(|_| true)))
            } else {
                match chain_selection::compare_against(blockchain.storage(), &tip_ref, &candidate) {
                    ComparisonResult::PreferCurrent => {
                        info!(logger, "create new branch");
                        A(B(future::ok(false)))
                    }
                    ComparisonResult::PreferCandidate => {
                        info!(logger, "switching to new candidate branch");
                        B(blockchain
                            .branches_mut()
                            .apply_or_create(candidate)
                            .and_then(move |branch| tip.swap(branch))
                            .map(|()| true))
                    }
                }
            }
        })
        .map_err(|_: std::convert::Infallible| unreachable!())
        .and_then(move |tip_updated| {
            if tip_updated {
                A(storage
                    .put_tag(MAIN_BRANCH_TAG.to_owned(), candidate_hash)
                    .map_err(|e| Error::with_chain(e, "Cannot update the main storage's tip")))
            } else {
                B(future::ok(()))
            }
        })
}

fn process_and_propagate_new_ref(
    logger: Logger,
    blockchain: Blockchain,
    tip: Tip,
    new_block_ref: Arc<Ref>,
    network_msg_box: MessageBox<NetworkMsg>,
) -> impl Future<Item = (), Error = Error> {
    let process_new_ref = process_new_ref(logger.clone(), blockchain, tip, new_block_ref.clone());

    process_new_ref.and_then(move |()| {
        debug!(logger, "propagating block to the network");
        let header = new_block_ref.header().clone();
        network_msg_box
            .send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
            .map_err(|_| "Cannot propagate block to network".into())
            .map(|_| ())
    })
}

pub fn process_leadership_block(
    logger: Logger,
    blockchain: Blockchain,
    block: Block,
) -> impl Future<Item = Arc<Ref>, Error = Error> {
    let end_blockchain = blockchain.clone();
    let header = block.header();
    let parent_hash = block.parent_id();
    let logger1 = logger.clone();
    let logger2 = logger.clone();
    // This is a trusted block from the leadership task,
    // so we can skip pre-validation.
    blockchain
        .get_ref(parent_hash)
        .and_then(move |parent| {
            if let Some(parent_ref) = parent {
                debug!(logger1, "processing block from leader event");
                Either::A(blockchain.post_check_header(header, parent_ref))
            } else {
                error!(
                    logger1,
                    "block from leader event does not have parent block in storage"
                );
                Either::B(future::err(
                    ErrorKind::MissingParentBlock(parent_hash).into(),
                ))
            }
        })
        .and_then(move |post_checked| {
            debug!(logger2, "apply and store block");
            end_blockchain.apply_and_store_block(post_checked, block)
        })
        .map_err(|err| Error::with_chain(err, "cannot process leadership block"))
        .map(move |e| {
            info!(logger, "block from leader event successfully stored");
            e
        })
}

fn process_block_announcement(
    blockchain: Blockchain,
    blockchain_tip: Tip,
    header: Header,
    node_id: NodeId,
    mut network_msg_box: MessageBox<NetworkMsg>,
    logger: Logger,
) -> impl Future<Item = (), Error = Error> {
    blockchain
        .pre_check_header(header, false)
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { .. } => {
                debug!(logger, "block is already present");
                Either::A(future::ok(()))
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                debug!(logger, "block is missing a locally stored parent");
                let to = header.hash();
                Either::B(
                    blockchain
                        .get_checkpoints(blockchain_tip.branch().clone())
                        .map(move |from| {
                            network_msg_box
                                .try_send(NetworkMsg::PullHeaders { node_id, from, to })
                                .unwrap_or_else(move |err| {
                                    error!(
                                        logger,
                                        "cannot send PullHeaders request to network: {}", err
                                    )
                                });
                        }),
                )
            }
            PreCheckedHeader::HeaderWithCache {
                header,
                parent_ref: _,
            } => {
                debug!(
                    logger,
                    "Announced block has a locally stored parent, fetch it"
                );
                network_msg_box
                    .try_send(NetworkMsg::GetNextBlock(node_id, header.hash()))
                    .unwrap_or_else(move |err| {
                        error!(
                            logger,
                            "cannot send GetNextBlock request to network: {}", err
                        )
                    });
                Either::A(future::ok(()))
            }
        })
        .map_err(|err| Error::with_chain(err, "cannot process block announcement"))
}

pub fn process_network_block(
    blockchain: Blockchain,
    candidate_forest: CandidateForest,
    block: Block,
    tx_msg_box: MessageBox<TransactionMsg>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    logger: Logger,
) -> impl Future<Item = Option<Arc<Ref>>, Error = chain::Error> {
    use futures::future::Either::{A, B};

    let header = block.header();
    blockchain
        .pre_check_header(header, false)
        .and_then(move |pre_checked| match pre_checked {
            PreCheckedHeader::AlreadyPresent { header, .. } => {
                debug!(
                    logger,
                    "block is already present";
                    "hash" => %header.hash(),
                    "parent" => %header.parent_id(),
                    "date" => %header.block_date(),
                );
                A(A(future::ok(None)))
            }
            PreCheckedHeader::MissingParent { header, .. } => {
                debug!(
                    logger,
                    "block is missing a locally stored parent, caching as candidate";
                    "hash" => %header.hash(),
                    "parent" => %header.parent_id(),
                    "date" => %header.block_date(),
                );
                A(B(candidate_forest.cache_block(block).map(|()| None)))
            }
            PreCheckedHeader::HeaderWithCache { parent_ref, .. } => {
                let post_check_and_apply = candidate_forest
                    .apply_block(block)
                    .and_then(move |blocks| {
                        check_and_apply_blocks(
                            blockchain,
                            parent_ref,
                            blocks,
                            tx_msg_box,
                            explorer_msg_box,
                            logger,
                        )
                    })
                    .map(Some);
                B(post_check_and_apply)
            }
        })
}

fn check_and_apply_blocks(
    blockchain: Blockchain,
    parent_ref: Arc<Ref>,
    blocks: Vec<Block>,
    tx_msg_box: MessageBox<TransactionMsg>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    logger: Logger,
) -> impl Future<Item = Arc<Ref>, Error = chain::Error> {
    let explorer_enabled = explorer_msg_box.is_some();
    stream::iter_ok(blocks).fold(parent_ref, move |parent_ref, block| {
        let blockchain1 = blockchain.clone();
        let mut tx_msg_box = tx_msg_box.clone();
        let mut explorer_msg_box = explorer_msg_box.clone();
        let logger = logger.clone();
        let header = block.header();
        blockchain
            .post_check_header(header, parent_ref)
            .and_then(move |post_checked| {
                let header = post_checked.header();
                debug!(
                    logger,
                    "applying block to storage";
                    "hash" => %header.hash(),
                    "parent" => %header.parent_id(),
                    "date" => %header.block_date(),
                );
                let mut block_for_explorer = if explorer_enabled {
                    Some(block.clone())
                } else {
                    None
                };
                let fragment_ids = block.fragments().map(|f| f.id()).collect::<Vec<_>>();
                blockchain1
                    .apply_and_store_block(post_checked, block)
                    .and_then(move |block_ref| {
                        try_request_fragment_removal(&mut tx_msg_box, fragment_ids, block_ref.header()).unwrap_or_else(|err| {
                            error!(logger, "cannot remove fragments from pool" ; "reason" => %err)
                        });
                        if let Some(msg_box) = explorer_msg_box.as_mut() {
                            msg_box
                                .try_send(ExplorerMsg::NewBlock(block_for_explorer.take().unwrap()))
                                .unwrap_or_else(|err| {
                                    error!(logger, "cannot add block to explorer: {}", err)
                                });
                        }
                        Ok(block_ref)
                    })
            })
    })
}

fn network_block_error_into_reply(err: chain::Error) -> intercom::Error {
    use super::chain::ErrorKind::*;

    match err.0 {
        Storage(e) => intercom::Error::failed(e),
        Ledger(e) => intercom::Error::failed_precondition(e),
        Block0(e) => intercom::Error::failed(e),
        MissingParentBlock(_) => intercom::Error::failed_precondition(err.to_string()),
        BlockHeaderVerificationFailed(_) => intercom::Error::invalid_argument(err.to_string()),
        _ => intercom::Error::failed(err.to_string()),
    }
}

fn chain_header_error_into_reply(err: candidate::Error) -> intercom::Error {
    use super::candidate::Error::*;

    // TODO: more detailed error case matching
    match err {
        Blockchain(e) => intercom::Error::failed(e.to_string()),
        EmptyHeaderStream => intercom::Error::invalid_argument(err.to_string()),
        MissingParentBlock(_) => intercom::Error::failed_precondition(err.to_string()),
        BrokenHeaderChain(_) => intercom::Error::invalid_argument(err.to_string()),
        HeaderChainVerificationFailed(e) => intercom::Error::invalid_argument(e),
        _ => intercom::Error::failed(err.to_string()),
    }
}
