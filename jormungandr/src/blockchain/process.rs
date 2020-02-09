use super::{
    candidate,
    chain::{self, AppliedBlock},
    chain_selection::{self, ComparisonResult},
    Blockchain, Error, ErrorKind, PreCheckedHeader, Ref, Tip, MAIN_BRANCH_TAG,
};
use crate::{
    blockcfg::{Block, FragmentId, Header},
    blockchain::Checkpoints,
    intercom::{self, BlockMsg, ExplorerMsg, NetworkMsg, PropagateMsg, TransactionMsg},
    log,
    network::p2p::Id as NodeId,
    stats_counter::StatsCounter,
    utils::{
        async_msg::{self, MessageBox, MessageQueue},
        fire_forget_scheduler::{
            FireForgetScheduler, FireForgetSchedulerConfig, FireForgetSchedulerFuture,
        },
        task::TokioServiceInfo,
    },
    HeaderHash,
};
use chain_core::property::{Block as _, Fragment as _, HasHeader as _, Header as _};
use jormungandr_lib::interfaces::FragmentStatus;

use futures::future::Either;
use slog::Logger;
use tokio::{
    prelude::*,
    timer::{timeout, Timeout},
};
use tokio_compat::prelude::*;

use std::{sync::Arc, time::Duration};

type TimeoutError = timeout::Error<Error>;
type PullHeadersScheduler = FireForgetScheduler<HeaderHash, NodeId, Checkpoints>;
type GetNextBlockScheduler = FireForgetScheduler<HeaderHash, NodeId, ()>;

const BRANCH_REPROCESSING_INTERVAL: Duration = Duration::from_secs(60);

const DEFAULT_TIMEOUT_PROCESS_LEADERSHIP: u64 = 5;
const DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT: u64 = 5;
const DEFAULT_TIMEOUT_PROCESS_BLOCKS: u64 = 60;
const DEFAULT_TIMEOUT_PROCESS_HEADERS: u64 = 60;

const PULL_HEADERS_SCHEDULER_CONFIG: FireForgetSchedulerConfig = FireForgetSchedulerConfig {
    max_running: 16,
    max_running_same_task: 2,
    command_channel_size: 1024,
    timeout: Duration::from_millis(500),
};

const GET_NEXT_BLOCK_SCHEDULER_CONFIG: FireForgetSchedulerConfig = FireForgetSchedulerConfig {
    max_running: 16,
    max_running_same_task: 2,
    command_channel_size: 1024,
    timeout: Duration::from_millis(500),
};

pub struct Process {
    pub blockchain: Blockchain,
    pub blockchain_tip: Tip,
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
        self.start_branch_reprocessing(&service_info);
        let pull_headers_scheduler = self.spawn_pull_headers_scheduler(&service_info);
        let get_next_block_scheduler = self.spawn_get_next_block_scheduler(&service_info);
        input.for_each(move |msg| {
            self.handle_input(
                &service_info,
                msg,
                &pull_headers_scheduler,
                &get_next_block_scheduler,
            );
            future::ok(())
        })
    }

    fn handle_input(
        &mut self,
        info: &TokioServiceInfo,
        input: BlockMsg,
        pull_headers_scheduler: &PullHeadersScheduler,
        get_next_block_scheduler: &GetNextBlockScheduler,
    ) {
        let blockchain = self.blockchain.clone();
        let blockchain_tip = self.blockchain_tip.clone();
        let network_msg_box = self.network_msgbox.clone();
        let explorer_msg_box = self.explorer_msgbox.clone();
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
                    "process leadership block",
                    Timeout::new(notify_explorer, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_LEADERSHIP))
                        .map_err(move |err: TimeoutError| {
                            error!(logger, "cannot process leadership block" ; "reason" => ?err)
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
                    pull_headers_scheduler.clone(),
                    get_next_block_scheduler.clone(),
                    logger.clone(),
                );

                info.spawn("process block announcement", Timeout::new(future, Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT)).map_err(move |err: TimeoutError| {
                    error!(logger, "cannot process block announcement" ; "reason" => ?err)
                }))
            }
            BlockMsg::NetworkBlocks(handle) => {
                info!(info.logger(), "receiving block stream from network");

                let logger = info.logger().clone();
                let get_next_block_scheduler = get_next_block_scheduler.clone();

                info.timeout_spawn_failable_std(
                    "process network blocks",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_BLOCKS),
                    process_network_blocks(
                        blockchain,
                        blockchain_tip,
                        tx_msg_box,
                        network_msg_box,
                        explorer_msg_box,
                        get_next_block_scheduler,
                        handle,
                        stats_counter,
                        logger,
                    ),
                );
            }
            BlockMsg::ChainHeaders(handle) => {
                info!(info.logger(), "receiving header stream from network");

                let (stream, reply) = handle.into_stream_and_reply();
                let logger = info.logger().new(o!(log::KEY_SUB_TASK => "chain_pull"));
                let logger_err1 = logger.clone();
                let logger_err2 = logger.clone();
                let schedule_logger = logger.clone();
                let mut pull_headers_scheduler = pull_headers_scheduler.clone();

                let future = candidate::advance_branch(blockchain, stream, logger)
                    .inspect(move |(header_ids, _)|
                        header_ids.iter()
                        .try_for_each(|header_id| pull_headers_scheduler.declare_completed(*header_id))
                        .unwrap_or_else(|e| error!(schedule_logger, "get blocks schedule completion failed"; "reason" => ?e)))
                    .then(move |resp| match resp {
                        Err(e) => {
                            info!(
                                logger_err1,
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
                    })
                    .timeout(Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_HEADERS))
                    .map_err(move |err: TimeoutError| {
                        error!(logger_err2, "cannot process network headers" ; "reason" => ?err)
                    });
                info.spawn("process network headers", future);
            }
        }
    }

    fn start_branch_reprocessing(&self, info: &TokioServiceInfo) {
        let tip = self.blockchain_tip.clone();
        let blockchain = self.blockchain.clone();
        let logger = info.logger().clone();

        info.run_periodic(
            "branch reprocessing",
            BRANCH_REPROCESSING_INTERVAL,
            move || reprocess_tip(logger.clone(), blockchain.clone(), tip.clone()),
        )
    }

    fn spawn_pull_headers_scheduler(&self, info: &TokioServiceInfo) -> PullHeadersScheduler {
        let network_msgbox = self.network_msgbox.clone();
        let scheduler_logger = info.logger().clone();
        let scheduler_future = FireForgetSchedulerFuture::new(
            &PULL_HEADERS_SCHEDULER_CONFIG,
            move |to, node_id, from| {
                network_msgbox
                    .clone()
                    .try_send(NetworkMsg::PullHeaders { node_id, from, to })
                    .unwrap_or_else(|e| {
                        error!(scheduler_logger, "cannot send PullHeaders request to network";
                        "reason" => e.to_string())
                    })
            },
        );
        let scheduler = scheduler_future.scheduler();
        let logger = info.logger().clone();
        let future = scheduler_future
            .map(|never| match never {})
            .map_err(move |e| error!(logger, "get blocks scheduling failed"; "reason" => ?e));
        info.spawn("pull headers scheduling", future);
        scheduler
    }

    fn spawn_get_next_block_scheduler(&self, info: &TokioServiceInfo) -> GetNextBlockScheduler {
        let network_msgbox = self.network_msgbox.clone();
        let scheduler_logger = info.logger().clone();
        let scheduler_future = FireForgetSchedulerFuture::new(
            &GET_NEXT_BLOCK_SCHEDULER_CONFIG,
            move |header_id, node_id, ()| {
                network_msgbox
                    .clone()
                    .try_send(NetworkMsg::GetNextBlock(node_id, header_id))
                    .unwrap_or_else(|e| {
                        error!(
                            scheduler_logger,
                            "cannot send GetNextBlock request to network"; "reason" => ?e
                        )
                    });
            },
        );
        let scheduler = scheduler_future.scheduler();
        let logger = info.logger().clone();
        let future = scheduler_future
            .map(|never| match never {})
            .map_err(move |e| error!(logger, "get next block scheduling failed"; "reason" => ?e));
        info.spawn("get next block scheduling", future);
        scheduler
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

/// this function will re-process the tip against the different branches
/// this is because a branch may have become more interesting with time
/// moving forward and branches may have been dismissed
pub fn reprocess_tip(
    logger: Logger,
    blockchain: Blockchain,
    tip: Tip,
) -> impl Future<Item = (), Error = Error> {
    let branches_future = blockchain.branches().branches();

    branches_future
        .join(tip.get_ref())
        .map(|(all, tip)| {
            all.into_iter()
                .filter(|r|
                    // remove our own tip so we don't apply it against itself
                    !Arc::ptr_eq(&r, &tip))
                .collect::<Vec<_>>()
        })
        .and_then(move |others| {
            stream::iter_ok(others)
                .for_each(move |other| {
                    process_new_ref(logger.clone(), blockchain.clone(), tip.clone(), other)
                })
                .into_future()
        })
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
    let storage = blockchain.storage().clone();

    tip.clone()
        .get_ref()
        .and_then(move |tip_ref| {
            if tip_ref.hash() == candidate.block_parent_hash() {
                info!(
                    logger,
                    "update current branch tip: {} -> {}",
                    tip_ref.header().description(),
                    candidate.header().description(),
                );
                A(A(tip.update_ref(candidate).map(|_| true)))
            } else {
                match chain_selection::compare_against(blockchain.storage(), &tip_ref, &candidate) {
                    ComparisonResult::PreferCurrent => {
                        info!(
                            logger,
                            "create new branch with tip {} | current-tip {}",
                            candidate.header().description(),
                            tip_ref.header().description(),
                        );
                        A(B(future::ok(false)))
                    }
                    ComparisonResult::PreferCandidate => {
                        info!(
                            logger,
                            "switching branch from {} to {}",
                            tip_ref.header().description(),
                            candidate.header().description(),
                        );
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
    let header = new_block_ref.header().clone();

    debug!(logger, "processing the new block and propagating"; "hash" => %header.hash());

    let process_new_ref = process_new_ref(logger.clone(), blockchain, tip, new_block_ref);

    process_new_ref.and_then(move |()| {
        debug!(logger, "propagating block to the network"; "hash" => %header.hash());
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
        .map(move |applied| {
            let new_ref = applied
                .new_ref()
                .expect("block from leadership must be unique");
            info!(logger, "block from leader event successfully stored");
            new_ref
        })
}

fn process_block_announcement(
    blockchain: Blockchain,
    blockchain_tip: Tip,
    header: Header,
    node_id: NodeId,
    mut pull_headers_scheduler: PullHeadersScheduler,
    mut get_next_block_scheduler: GetNextBlockScheduler,
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
                        .get_checkpoints(blockchain_tip.branch())
                        .map(move |from| {
                            pull_headers_scheduler
                                .schedule(to, node_id, from)
                                .unwrap_or_else(move |err| {
                                    error!(
                                        logger,
                                        "cannot schedule pulling headers"; "reason" => ?err
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
                get_next_block_scheduler
                    .schedule(header.id(), node_id, ())
                    .unwrap_or_else(move |err| {
                        error!(
                            logger,
                            "cannot schedule getting next block"; "reason" => ?err
                        )
                    });
                Either::A(future::ok(()))
            }
        })
        .map_err(|err| Error::with_chain(err, "cannot process block announcement"))
}

async fn process_network_blocks(
    blockchain: Blockchain,
    blockchain_tip: Tip,
    mut tx_msg_box: MessageBox<TransactionMsg>,
    network_msg_box: MessageBox<NetworkMsg>,
    mut explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    mut get_next_block_scheduler: GetNextBlockScheduler,
    handle: intercom::RequestStreamHandle<Block, ()>,
    stats_counter: StatsCounter,
    logger: Logger,
) -> Result<(), Error> {
    let (stream, reply) = handle.into_stream_and_reply();
    let mut stream = stream.map_err(|()| Error::from("Error while processing block input stream"));
    let mut candidate = None;

    let maybe_updated: Option<Arc<Ref>> = loop {
        let (maybe_block, new_stream) = stream.into_future().map_err(|(e, _)| e).compat().await?;
        match maybe_block {
            Some(block) => {
                let res = process_network_block(
                    &blockchain,
                    block,
                    &mut tx_msg_box,
                    explorer_msg_box.as_mut(),
                    &mut get_next_block_scheduler,
                    &logger,
                )
                .await;
                match res {
                    Ok(Some(r)) => {
                        stats_counter.add_block_recv_cnt(1);
                        stream = new_stream;
                        candidate = Some(r);
                    }
                    Ok(None) => {
                        reply.reply_ok(());
                        break candidate;
                    }
                    Err(e) => {
                        info!(
                            logger,
                            "validation of an incoming block failed";
                            "reason" => ?e,
                        );
                        reply.reply_error(network_block_error_into_reply(e));
                        break candidate;
                    }
                }
            }
            None => {
                reply.reply_ok(());
                break candidate;
            }
        }
    };

    match maybe_updated {
        Some(new_block_ref) => {
            let r = process_and_propagate_new_ref(
                logger,
                blockchain,
                blockchain_tip,
                Arc::clone(&new_block_ref),
                network_msg_box,
            )
            .compat()
            .await?;
            Ok(r)
        }
        None => Ok(()),
    }
}

async fn process_network_block(
    blockchain: &Blockchain,
    block: Block,
    tx_msg_box: &mut MessageBox<TransactionMsg>,
    explorer_msg_box: Option<&mut MessageBox<ExplorerMsg>>,
    get_next_block_scheduler: &mut GetNextBlockScheduler,
    logger: &Logger,
) -> Result<Option<Arc<Ref>>, chain::Error> {
    get_next_block_scheduler
        .declare_completed(block.id())
        .unwrap_or_else(
            |e| error!(logger, "get next block schedule completion failed"; "reason" => ?e),
        );
    let header = block.header();
    let pre_checked = blockchain.pre_check_header(header, false).compat().await?;
    match pre_checked {
        PreCheckedHeader::AlreadyPresent { header, .. } => {
            debug!(
                logger,
                "block is already present";
                "hash" => %header.hash(),
                "parent" => %header.parent_id(),
                "date" => %header.block_date(),
            );
            Ok(None)
        }
        PreCheckedHeader::MissingParent { header, .. } => {
            let parent_hash = header.parent_id();
            debug!(
                logger,
                "block is missing a locally stored parent";
                "hash" => %header.hash(),
                "parent" => %parent_hash,
                "date" => %header.block_date(),
            );
            Err(ErrorKind::MissingParentBlock(parent_hash).into())
        }
        PreCheckedHeader::HeaderWithCache { parent_ref, .. } => {
            let r = check_and_apply_block(
                blockchain,
                parent_ref,
                block,
                tx_msg_box,
                explorer_msg_box,
                logger,
            )
            .await;
            r
        }
    }
}

async fn check_and_apply_block(
    blockchain: &Blockchain,
    parent_ref: Arc<Ref>,
    block: Block,
    tx_msg_box: &mut MessageBox<TransactionMsg>,
    explorer_msg_box: Option<&mut MessageBox<ExplorerMsg>>,
    logger: &Logger,
) -> Result<Option<Arc<Ref>>, chain::Error> {
    let explorer_enabled = explorer_msg_box.is_some();
    let post_checked = blockchain
        .post_check_header(block.header(), parent_ref)
        .compat()
        .await?;
    let header = post_checked.header();
    let block_hash = header.hash();
    debug!(
        logger,
        "applying block to storage";
        "hash" => %block_hash,
        "parent" => %header.parent_id(),
        "date" => %header.block_date(),
    );
    let mut block_for_explorer = if explorer_enabled {
        Some(block.clone())
    } else {
        None
    };
    let fragment_ids = block.fragments().map(|f| f.id()).collect::<Vec<_>>();
    let applied_block = blockchain
        .apply_and_store_block(post_checked, block)
        .compat()
        .await?;
    if let AppliedBlock::New(block_ref) = applied_block {
        let header = block_ref.header();
        debug!(
            logger,
            "applied block to storage";
            "hash" => %block_hash,
            "parent" => %header.parent_id(),
            "date" => %header.block_date(),
        );
        try_request_fragment_removal(tx_msg_box, fragment_ids, header).unwrap_or_else(
            |err| error!(logger, "cannot remove fragments from pool" ; "reason" => %err),
        );
        if let Some(msg_box) = explorer_msg_box {
            msg_box
                .try_send(ExplorerMsg::NewBlock(block_for_explorer.take().unwrap()))
                .unwrap_or_else(|err| error!(logger, "cannot add block to explorer: {}", err));
        }
        Ok(Some(block_ref))
    } else {
        debug!(
            logger,
            "block is already present in storage, not applied";
            "hash" => %block_hash,
        );
        Ok(None)
    }
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
