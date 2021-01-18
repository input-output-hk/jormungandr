use super::{
    candidate,
    chain::{self, AppliedBlock, CheckHeaderProof},
    chain_selection::{self, ComparisonResult},
    Blockchain, Error, ErrorKind, PreCheckedHeader, Ref, Tip, MAIN_BRANCH_TAG,
};
use crate::{
    blockcfg::{Block, FragmentId, Header, HeaderHash},
    blockchain::Checkpoints,
    intercom::{self, BlockMsg, ExplorerMsg, NetworkMsg, PropagateMsg, TransactionMsg},
    log,
    network::p2p::Address,
    stats_counter::StatsCounter,
    utils::{
        async_msg::{self, MessageBox, MessageQueue},
        fire_forget_scheduler::{
            FireForgetScheduler, FireForgetSchedulerConfig, FireForgetSchedulerFuture,
        },
        task::TokioServiceInfo,
    },
};
use chain_core::property::{Block as _, Fragment as _, HasHeader as _, Header as _};
use jormungandr_lib::interfaces::FragmentStatus;

use futures::prelude::*;
use slog::Logger;

use std::{sync::Arc, time::Duration};

type PullHeadersScheduler = FireForgetScheduler<HeaderHash, Address, Checkpoints>;
type GetNextBlockScheduler = FireForgetScheduler<HeaderHash, Address, ()>;

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
    pub async fn start(
        mut self,
        service_info: TokioServiceInfo,
        mut input: MessageQueue<BlockMsg>,
    ) {
        self.start_branch_reprocessing(&service_info);
        self.start_garbage_collector(&service_info);
        let pull_headers_scheduler = self.spawn_pull_headers_scheduler(&service_info);
        let get_next_block_scheduler = self.spawn_get_next_block_scheduler(&service_info);
        while let Some(msg) = input.next().await {
            self.handle_input(
                &service_info,
                msg,
                &pull_headers_scheduler,
                &get_next_block_scheduler,
            );
        }
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
        let tx_msg_box = self.fragment_msgbox.clone();
        let stats_counter = self.stats_counter.clone();

        match input {
            BlockMsg::LeadershipBlock(block) => {
                let logger = info.logger().new(o!(
                    "hash" => block.header.hash().to_string(),
                    "parent" => block.header.parent_id().to_string(),
                    "date" => block.header.block_date().to_string()));

                info!(logger, "receiving block from leadership service");

                info.timeout_spawn_fallible(
                    "process leadership block",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_LEADERSHIP),
                    process_leadership_block(
                        logger,
                        blockchain,
                        blockchain_tip,
                        tx_msg_box,
                        network_msg_box,
                        explorer_msg_box,
                        block,
                        stats_counter,
                    ),
                )
            }
            BlockMsg::AnnouncedBlock(header, node_id) => {
                let logger = info.logger().new(o!(
                    "hash" => header.hash().to_string(),
                    "parent" => header.parent_id().to_string(),
                    "date" => header.block_date().to_string(),
                    "peer" => node_id.to_string()));

                info!(logger, "received block announcement from network");

                info.timeout_spawn_fallible(
                    "process block announcement",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT),
                    process_block_announcement(
                        blockchain,
                        blockchain_tip,
                        header,
                        node_id,
                        pull_headers_scheduler.clone(),
                        get_next_block_scheduler.clone(),
                        logger.clone(),
                    ),
                )
            }
            BlockMsg::NetworkBlocks(handle) => {
                info!(info.logger(), "receiving block stream from network");

                let logger = info.logger().clone();
                let get_next_block_scheduler = get_next_block_scheduler.clone();

                info.timeout_spawn_fallible(
                    "process network blocks",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_BLOCKS),
                    process_network_blocks(
                        self.blockchain.clone(),
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

                let logger = info.logger().new(o!(log::KEY_SUB_TASK => "chain_pull"));
                let pull_headers_scheduler = pull_headers_scheduler.clone();

                info.timeout_spawn(
                    "process network headers",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_HEADERS),
                    process_chain_headers(
                        logger,
                        blockchain,
                        handle,
                        pull_headers_scheduler,
                        network_msg_box,
                    ),
                );
            }
        }
    }

    fn start_branch_reprocessing(&self, info: &TokioServiceInfo) {
        let tip = self.blockchain_tip.clone();
        let blockchain = self.blockchain.clone();
        let explorer = self.explorer_msgbox.clone();
        let logger = info.logger().clone();

        info.run_periodic_fallible(
            "branch reprocessing",
            BRANCH_REPROCESSING_INTERVAL,
            move || {
                reprocess_tip(
                    logger.clone(),
                    blockchain.clone(),
                    tip.clone(),
                    explorer.clone(),
                )
            },
        )
    }

    fn start_garbage_collector(&self, info: &TokioServiceInfo) {
        let blockchain = self.blockchain.clone();
        let tip = self.blockchain_tip.clone();

        async fn blockchain_gc(blockchain: Blockchain, tip: Tip) -> chain::Result<()> {
            blockchain.gc(tip.get_ref().await).await
        }

        info.run_periodic_fallible(
            "collect stale branches",
            self.garbage_collection_interval,
            move || blockchain_gc(blockchain.clone(), tip.clone()),
        )
    }

    fn spawn_pull_headers_scheduler(&self, info: &TokioServiceInfo) -> PullHeadersScheduler {
        let network_msgbox = self.network_msgbox.clone();
        let scheduler_logger = info.logger().clone();
        let scheduler_future = FireForgetSchedulerFuture::new(
            &PULL_HEADERS_SCHEDULER_CONFIG,
            move |to, node_address, from| {
                network_msgbox
                    .clone()
                    .try_send(NetworkMsg::PullHeaders {
                        node_address,
                        from,
                        to,
                    })
                    .unwrap_or_else(|e| {
                        error!(scheduler_logger, "cannot send PullHeaders request to network";
                        "reason" => e.to_string())
                    })
            },
        );
        let scheduler = scheduler_future.scheduler();
        let logger = info.logger().clone();
        let future = scheduler_future
            .map_err(move |e| error!(logger, "get blocks scheduling failed"; "reason" => ?e));
        info.spawn_fallible("pull headers scheduling", future);
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
            .map_err(move |e| error!(logger, "get next block scheduling failed"; "reason" => ?e));
        info.spawn_fallible("get next block scheduling", future);
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
async fn reprocess_tip(
    logger: Logger,
    mut blockchain: Blockchain,
    tip: Tip,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
) -> Result<(), Error> {
    let branches: Vec<Arc<Ref>> = blockchain.branches().branches().await;

    let tip_as_ref = tip.get_ref().await;

    let others = branches
        .iter()
        .filter(|r| !Arc::ptr_eq(&r, &tip_as_ref))
        .collect::<Vec<_>>();

    for other in others {
        process_new_ref(
            &logger,
            &mut blockchain,
            tip.clone(),
            Arc::clone(other),
            explorer_msg_box.clone(),
        )
        .await?
    }

    Ok(())
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
pub async fn process_new_ref(
    blockchain: &mut Blockchain,
    mut tip: Tip,
    candidate: Arc<Ref>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
) -> Result<(), Error> {
    let candidate_hash = candidate.hash();
    let tip_ref = tip.get_ref().await;

    match chain_selection::compare_against(blockchain.storage(), &tip_ref, &candidate) {
        ComparisonResult::PreferCurrent => {
            tracing::info!(
                "create new branch with tip {} | current-tip {}",
                candidate.header().description(),
                tip_ref.header().description(),
            );
        }
        ComparisonResult::PreferCandidate => {
            if tip_ref.hash() == candidate.block_parent_hash() {
                tracing::info!(
                    "update current branch tip: {} -> {}",
                    tip_ref.header().description(),
                    candidate.header().description(),
                );

                blockchain
                    .storage()
                    .put_tag(MAIN_BRANCH_TAG, candidate_hash)
                    .map_err(|e| Error::with_chain(e, "Cannot update the main storage's tip"))?;

                tip.update_ref(candidate).await;
            } else {
                tracing::info!(
                    "switching branch from {} to {}",
                    tip_ref.header().description(),
                    candidate.header().description(),
                );

                blockchain
                    .storage()
                    .put_tag(MAIN_BRANCH_TAG, candidate_hash)
                    .map_err(|e| Error::with_chain(e, "Cannot update the main storage's tip"))?;

                let branch = blockchain.branches_mut().apply_or_create(candidate).await;
                tip.swap(branch).await;
            }

            if let Some(mut msg_box) = explorer_msg_box {
                msg_box
                    .send(ExplorerMsg::NewTip(candidate_hash))
                    .await
                    .unwrap_or_else(|err| {
                        error!(logger, "cannot send new tip to explorer: {}", err)
                    });
            }
        }
    }

    Ok(())
}

async fn process_and_propagate_new_ref(
    logger: &Logger,
    blockchain: &mut Blockchain,
    tip: Tip,
    new_block_ref: Arc<Ref>,
    mut network_msg_box: MessageBox<NetworkMsg>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
) -> Result<(), Error> {
    let header = new_block_ref.header().clone();
    debug!(logger, "processing the new block and propagating");

    process_new_ref(logger, blockchain, tip, new_block_ref, explorer_msg_box).await?;

    debug!(logger, "propagating block to the network");
    network_msg_box
        .send(NetworkMsg::Propagate(PropagateMsg::Block(header)))
        .await
        .map_err(|_| "Cannot propagate block to network".into())
        .map(|_| ())
}

#[allow(clippy::too_many_arguments)]
async fn process_leadership_block(
    logger: Logger,
    mut blockchain: Blockchain,
    blockchain_tip: Tip,
    mut tx_msg_box: MessageBox<TransactionMsg>,
    network_msg_box: MessageBox<NetworkMsg>,
    explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    block: Block,
    stats_counter: StatsCounter,
) -> Result<(), Error> {
    let new_block_ref =
        process_leadership_block_inner(&logger, &mut blockchain, block.clone()).await?;

    let fragments = block.fragments().map(|f| f.id()).collect();

    trace!(logger, "updating fragments log");
    try_request_fragment_removal(&mut tx_msg_box, fragments, new_block_ref.header())
        .map_err(|_| "cannot remove fragments from pool".to_string())?;

    process_and_propagate_new_ref(
        &logger,
        &mut blockchain,
        blockchain_tip,
        Arc::clone(&new_block_ref),
        network_msg_box,
        explorer_msg_box.clone(),
    )
    .await?;

    // Track block as new new tip block
    stats_counter.set_tip_block(Arc::new(block.clone()));

    if let Some(mut msg_box) = explorer_msg_box {
        msg_box
            .send(ExplorerMsg::NewBlock(block))
            .await
            .map_err(|_| "Cannot propagate block to explorer".to_string())?;
    }
    Ok(())
}

async fn process_leadership_block_inner(
    logger: &Logger,
    blockchain: &mut Blockchain,
    block: Block,
) -> Result<Arc<Ref>, Error> {
    let header = block.header();
    let parent_hash = block.parent_id();
    // This is a trusted block from the leadership task,
    // so we can skip pre-validation.
    let parent = blockchain.get_ref(parent_hash).await?;

    let post_checked = if let Some(parent_ref) = parent {
        debug!(logger, "processing block from leader event");
        blockchain
            .post_check_header(header, parent_ref, CheckHeaderProof::Enabled)
            .await?
    } else {
        error!(
            logger,
            "block from leader event does not have parent block in storage"
        );
        return Err(ErrorKind::MissingParentBlock(parent_hash).into());
    };

    debug!(logger, "apply and store block");
    let applied = blockchain
        .apply_and_store_block(post_checked, block)
        .await
        .map_err(|err| Error::with_chain(err, "cannot process leadership block"))?;
    let new_ref = applied
        .new_ref()
        .expect("block from leadership must be unique");
    info!(logger, "block from leader event successfully stored");
    Ok(new_ref)
}

async fn process_block_announcement(
    blockchain: Blockchain,
    blockchain_tip: Tip,
    header: Header,
    node_id: Address,
    mut pull_headers_scheduler: PullHeadersScheduler,
    mut get_next_block_scheduler: GetNextBlockScheduler,
    logger: Logger,
) -> Result<(), Error> {
    let pre_checked = blockchain
        .pre_check_header(header, false)
        .await
        .map_err(|err| Error::with_chain(err, "cannot process block announcement"))?;
    match pre_checked {
        PreCheckedHeader::AlreadyPresent { .. } => {
            debug!(logger, "block is already present");
            Ok(())
        }
        PreCheckedHeader::MissingParent { header, .. } => {
            debug!(logger, "block is missing a locally stored parent");
            let to = header.hash();
            let from = blockchain.get_checkpoints(blockchain_tip.branch()).await;
            pull_headers_scheduler
                .schedule(to, node_id, from)
                .unwrap_or_else(move |err| {
                    error!(
                        logger,
                        "cannot schedule pulling headers"; "reason" => ?err
                    )
                });
            Ok(())
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
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_network_blocks(
    mut blockchain: Blockchain,
    blockchain_tip: Tip,
    mut tx_msg_box: MessageBox<TransactionMsg>,
    network_msg_box: MessageBox<NetworkMsg>,
    mut explorer_msg_box: Option<MessageBox<ExplorerMsg>>,
    mut get_next_block_scheduler: GetNextBlockScheduler,
    handle: intercom::RequestStreamHandle<Block, ()>,
    stats_counter: StatsCounter,
    logger: Logger,
) -> Result<(), Error> {
    let (mut stream, reply) = handle.into_stream_and_reply();
    let mut candidate = None;
    let mut latest_block: Option<Arc<Block>> = None;

    let maybe_updated: Option<Arc<Ref>> = loop {
        let (maybe_block, stream_tail) = stream.into_future().await;
        match maybe_block {
            Some(block) => {
                latest_block = Some(Arc::new(block.clone()));
                let res = process_network_block(
                    &blockchain,
                    block.clone(),
                    &mut tx_msg_box,
                    explorer_msg_box.as_mut(),
                    &mut get_next_block_scheduler,
                    &logger,
                )
                .await;
                match res {
                    Ok(Some(r)) => {
                        stats_counter.add_block_recv_cnt(1);
                        stream = stream_tail;
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
            process_and_propagate_new_ref(
                &logger,
                &mut blockchain,
                blockchain_tip,
                Arc::clone(&new_block_ref),
                network_msg_box,
                explorer_msg_box,
            )
            .await?;

            // Add block if found
            if let Some(b) = latest_block {
                stats_counter.set_tip_block(b);
            };
            Ok(())
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
    let pre_checked = blockchain.pre_check_header(header, false).await?;
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
        .post_check_header(block.header(), parent_ref, CheckHeaderProof::Enabled)
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

async fn process_chain_headers(
    logger: Logger,
    blockchain: Blockchain,
    handle: intercom::RequestStreamHandle<Header, ()>,
    mut pull_headers_scheduler: PullHeadersScheduler,
    mut network_msg_box: MessageBox<NetworkMsg>,
) {
    let (stream, reply) = handle.into_stream_and_reply();
    match candidate::advance_branch(blockchain, stream, logger.clone()).await {
        Err(e) => {
            info!(
                logger,
                "error processing an incoming header stream";
                "reason" => %e,
            );
            reply.reply_error(chain_header_error_into_reply(e));
        }
        Ok((header_ids, _maybe_remainder)) => {
            header_ids
                .iter()
                .try_for_each(|header_id| pull_headers_scheduler.declare_completed(*header_id))
                .unwrap_or_else(
                    |e| error!(logger, "get blocks schedule completion failed"; "reason" => ?e),
                );

            if header_ids.is_empty() {
            } else {
                network_msg_box
                    .send(NetworkMsg::GetBlocks(header_ids))
                    .await
                    .map_err(|_| error!(logger, "cannot request blocks from network"))
                    .map(|_| ())
                    .unwrap();

                reply.reply_ok(())
                // TODO: if the stream is not ended, resume processing
                // after more blocks arrive
            }
        }
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
    }
}
