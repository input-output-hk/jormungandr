use super::{
    candidate,
    chain::{self, AppliedBlock, CheckHeaderProof, LeadershipBlock},
    tip::TipUpdater,
    Blockchain, Error, PreCheckedHeader, Ref, Tip,
};
use crate::{
    blockcfg::{Block, Header, HeaderHash},
    blockchain::Checkpoints,
    intercom::{self, BlockMsg, NetworkMsg, PropagateMsg, TransactionMsg, WatchMsg},
    metrics::{Metrics, MetricsBackend},
    topology::NodeId,
    utils::{
        async_msg::{self, MessageBox, MessageQueue},
        fire_forget_scheduler::{
            FireForgetScheduler, FireForgetSchedulerConfig, FireForgetSchedulerFuture,
        },
        task::TokioServiceInfo,
    },
};
use chain_core::property::{Block as _, Header as _};
use futures::prelude::*;
use std::{sync::Arc, time::Duration};
use tracing::{span, Level};
use tracing_futures::Instrument;

type PullHeadersScheduler = FireForgetScheduler<HeaderHash, NodeId, Checkpoints>;
type GetNextBlockScheduler = FireForgetScheduler<HeaderHash, NodeId, ()>;

const TIP_UPDATE_QUEUE_SIZE: usize = 10;

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

pub struct TaskData {
    pub blockchain: Blockchain,
    pub blockchain_tip: Tip,
    pub stats_counter: Metrics,
    pub network_msgbox: MessageBox<NetworkMsg>,
    pub fragment_msgbox: MessageBox<TransactionMsg>,
    pub watch_msgbox: MessageBox<WatchMsg>,
    pub garbage_collection_interval: Duration,
}

/// The blockchain process is comprised mainly of two parts:
///
/// * Bookkeeping of all blocks known to the node:
///     This is the most resource heavy operation but can be parallelized depending on the chain structure.
/// * Tip selection and update:
///     Tip updates must be serialized to avoid inconsistent states but are very light on resources.
///     No performance penalty should come from this synchronization point.
struct Process {
    blockchain: Blockchain,
    blockchain_tip: Tip,
    stats_counter: Metrics,
    network_msgbox: MessageBox<NetworkMsg>,
    fragment_msgbox: MessageBox<TransactionMsg>,
    watch_msgbox: MessageBox<WatchMsg>,
    garbage_collection_interval: Duration,
    tip_update_mbox: MessageBox<Arc<Ref>>,
    pull_headers_scheduler: PullHeadersScheduler,
    get_next_block_scheduler: GetNextBlockScheduler,
    service_info: TokioServiceInfo,
}

fn spawn_pull_headers_scheduler(
    network_mbox: MessageBox<NetworkMsg>,
    info: &TokioServiceInfo,
) -> PullHeadersScheduler {
    let scheduler_future = FireForgetSchedulerFuture::new(
        &PULL_HEADERS_SCHEDULER_CONFIG,
        move |to, node_id, from| {
            network_mbox
                .clone()
                .try_send(NetworkMsg::PullHeaders {
                    node_id,
                    from,
                    to,
                })
                .unwrap_or_else(|e| {
                    tracing::error!(reason = %e.to_string(), "cannot send PullHeaders request to network")
                })
        },
    );
    let scheduler = scheduler_future.scheduler();
    let future = scheduler_future
        .map_err(move |e| tracing::error!(reason = ?e, "pull headers scheduling failed"));
    info.spawn_fallible("pull headers scheduling", future);
    scheduler
}

fn spawn_get_next_block_scheduler(
    network_mbox: MessageBox<NetworkMsg>,
    info: &TokioServiceInfo,
) -> GetNextBlockScheduler {
    let scheduler_future = FireForgetSchedulerFuture::new(
        &GET_NEXT_BLOCK_SCHEDULER_CONFIG,
        move |header_id, node_id, ()| {
            network_mbox
                .clone()
                .try_send(NetworkMsg::GetNextBlock(node_id, header_id))
                .unwrap_or_else(|e| {
                    tracing::error!(
                        reason = ?e,
                        "cannot send GetNextBlock request to network"
                    )
                });
        },
    );
    let scheduler = scheduler_future.scheduler();
    let future = scheduler_future
        .map_err(move |e| tracing::error!(reason = ?e, "get next block scheduling failed"));
    info.spawn_fallible("get next block scheduling", future);
    scheduler
}

pub async fn start(
    task_data: TaskData,
    service_info: TokioServiceInfo,
    input: MessageQueue<BlockMsg>,
) {
    let TaskData {
        blockchain,
        blockchain_tip,
        stats_counter,
        network_msgbox,
        fragment_msgbox,
        garbage_collection_interval,
        watch_msgbox,
    } = task_data;

    let (tip_update_mbox, tip_update_queue) = async_msg::channel(TIP_UPDATE_QUEUE_SIZE);
    let pull_headers_scheduler =
        spawn_pull_headers_scheduler(network_msgbox.clone(), &service_info);
    let get_next_block_scheduler =
        spawn_get_next_block_scheduler(network_msgbox.clone(), &service_info);

    Process {
        blockchain,
        blockchain_tip,
        stats_counter,
        network_msgbox,
        fragment_msgbox,
        watch_msgbox,
        garbage_collection_interval,
        tip_update_mbox,
        pull_headers_scheduler,
        get_next_block_scheduler,
        service_info,
    }
    .start(input, tip_update_queue)
    .await
}

impl Process {
    async fn start(
        mut self,
        mut input: MessageQueue<BlockMsg>,
        tip_update_queue: MessageQueue<Arc<Ref>>,
    ) {
        self.start_garbage_collector(&self.service_info);

        let mut tip_updater = TipUpdater::new(
            self.blockchain_tip.clone(),
            self.blockchain.clone(),
            Some(self.fragment_msgbox.clone()),
            Some(self.watch_msgbox.clone()),
            self.stats_counter.clone(),
        );

        self.service_info.spawn("tip updater", async move {
            tip_updater.run(tip_update_queue).await
        });

        while let Some(input) = input.next().await {
            self.handle_input(input);
        }
    }

    fn handle_input(&mut self, input: BlockMsg) {
        let blockchain = self.blockchain.clone();
        let blockchain_tip = self.blockchain_tip.clone();
        let network_msg_box = self.network_msgbox.clone();
        let watch_msg_box = self.watch_msgbox.clone();
        let stats_counter = self.stats_counter.clone();
        tracing::trace!("handling new blockchain task item");
        match input {
            BlockMsg::LeadershipBlock(leadership_block) => {
                let span = span!(
                    parent: self.service_info.span(),
                    Level::DEBUG,
                    "process_leadership_block",
                    hash = %leadership_block.block.header().hash(),
                    parent = %leadership_block.block.header().parent_id(),
                    date = %leadership_block.block.header().block_date()
                );
                let _enter = span.enter();
                tracing::debug!("receiving block from leadership service");

                self.service_info.timeout_spawn_fallible(
                    "process leadership block",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_LEADERSHIP),
                    process_leadership_block(
                        blockchain,
                        self.tip_update_mbox.clone(),
                        network_msg_box,
                        watch_msg_box,
                        *leadership_block,
                    )
                    .instrument(span.clone()),
                );
            }
            BlockMsg::AnnouncedBlock(header, node_id) => {
                let span = span!(
                    parent: self.service_info.span(),
                    Level::DEBUG,
                    "process_announced_block",
                    hash = %header.hash(),
                    parent = %header.parent_id(),
                    date = %header.block_date(),
                    %node_id
                );
                let _enter = span.enter();
                tracing::debug!("received block announcement from network");

                self.service_info.timeout_spawn_fallible(
                    "process block announcement",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_ANNOUNCEMENT),
                    process_block_announcement(
                        blockchain,
                        blockchain_tip,
                        *header,
                        node_id,
                        self.pull_headers_scheduler.clone(),
                        self.get_next_block_scheduler.clone(),
                    )
                    .instrument(span.clone()),
                )
            }
            BlockMsg::NetworkBlocks(handle) => {
                let span = span!(
                    parent: self.service_info.span(),
                    Level::DEBUG,
                    "process_network_blocks",
                );
                let _guard = span.enter();
                tracing::debug!("receiving block stream from network");

                self.service_info.timeout_spawn_fallible(
                    "process network blocks",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_BLOCKS),
                    process_network_blocks(
                        self.blockchain.clone(),
                        self.tip_update_mbox.clone(),
                        network_msg_box,
                        watch_msg_box,
                        self.get_next_block_scheduler.clone(),
                        handle,
                        stats_counter,
                    )
                    .instrument(span.clone()),
                );
            }
            BlockMsg::ChainHeaders(handle) => {
                let span = span!(parent: self.service_info.span(), Level::DEBUG, "process_chain_headers", sub_task = "chain_pull");
                let _enter = span.enter();
                tracing::debug!("receiving header stream from network");

                self.service_info.timeout_spawn(
                    "process network headers",
                    Duration::from_secs(DEFAULT_TIMEOUT_PROCESS_HEADERS),
                    process_chain_headers(
                        blockchain,
                        handle,
                        self.pull_headers_scheduler.clone(),
                        network_msg_box,
                    )
                    .instrument(span.clone()),
                );
            }
        }
        tracing::trace!("item handling finished");
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
}

async fn process_and_propagate_new_ref(
    new_block_ref: Arc<Ref>,
    mut tip_update_mbox: MessageBox<Arc<Ref>>,
    mut network_msg_box: MessageBox<NetworkMsg>,
) -> chain::Result<()> {
    let header = new_block_ref.header().clone();
    let span = span!(Level::DEBUG, "process_and_propagate_new_ref", block = %header.hash());

    async {
        tracing::debug!("processing the new block and propagating");
        // Even if this fails because the queue is full we periodically recompute the tip
        tip_update_mbox
            .try_send(new_block_ref)
            .unwrap_or_else(|err| {
                tracing::error!(
                    "cannot send new ref to be evaluated as candidate tip: {}",
                    err
                )
            });

        tracing::debug!("propagating block to the network");

        network_msg_box
            .send(NetworkMsg::Propagate(Box::new(PropagateMsg::Block(
                Box::new(header),
            ))))
            .await?;
        Ok::<(), Error>(())
    }
    .instrument(span)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_leadership_block(
    mut blockchain: Blockchain,
    tip_update_mbox: MessageBox<Arc<Ref>>,
    network_msg_box: MessageBox<NetworkMsg>,
    mut watch_msg_box: MessageBox<WatchMsg>,
    leadership_block: LeadershipBlock,
) -> chain::Result<()> {
    let block = leadership_block.block.clone();
    let new_block_ref = process_leadership_block_inner(&mut blockchain, leadership_block).await?;

    watch_msg_box
        .send(WatchMsg::NewBlock(block.clone()))
        .await?;

    process_and_propagate_new_ref(Arc::clone(&new_block_ref), tip_update_mbox, network_msg_box)
        .await?;

    Ok(())
}

async fn process_leadership_block_inner(
    blockchain: &mut Blockchain,
    leadership_block: LeadershipBlock,
) -> Result<Arc<Ref>, Error> {
    let applied = blockchain
        .apply_and_store_leadership_block(leadership_block)
        .await?;
    let new_ref = applied
        .new_ref()
        .expect("block from leadership must be unique");
    tracing::info!("block from leader event successfully stored");
    Ok(new_ref)
}

async fn process_block_announcement(
    blockchain: Blockchain,
    blockchain_tip: Tip,
    header: Header,
    node_id: NodeId,
    mut pull_headers_scheduler: PullHeadersScheduler,
    mut get_next_block_scheduler: GetNextBlockScheduler,
) -> Result<(), Error> {
    let pre_checked = blockchain.pre_check_header(header, false).await?;
    match pre_checked {
        PreCheckedHeader::AlreadyPresent { .. } => {
            tracing::debug!("block is already present");
            Ok(())
        }
        PreCheckedHeader::MissingParent { header, .. } => {
            tracing::debug!("block is missing a locally stored parent");
            let to = header.hash();
            let from = blockchain.get_checkpoints(&blockchain_tip.branch().await);
            pull_headers_scheduler
                .schedule(to, node_id, from)
                .unwrap_or_else(move |err| {
                    tracing::error!(
                         reason = ?err,
                        "cannot schedule pulling headers"
                    )
                });
            Ok(())
        }
        PreCheckedHeader::HeaderWithCache {
            header,
            parent_ref: _,
        } => {
            tracing::debug!("Announced block has a locally stored parent, fetch it");
            get_next_block_scheduler
                .schedule(header.id(), node_id, ())
                .unwrap_or_else(move |err| {
                    tracing::error!(
                         reason = ?err,
                        "cannot schedule getting next block"
                    )
                });
            Ok(())
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_network_blocks(
    blockchain: Blockchain,
    tip_update_mbox: MessageBox<Arc<Ref>>,
    network_msg_box: MessageBox<NetworkMsg>,
    mut watch_msg_box: MessageBox<WatchMsg>,
    mut get_next_block_scheduler: GetNextBlockScheduler,
    handle: intercom::RequestStreamHandle<Block, ()>,
    stats_counter: Metrics,
) -> Result<(), Error> {
    let (mut stream, reply) = handle.into_stream_and_reply();
    let mut candidate = None;

    let maybe_updated: Option<Arc<Ref>> = loop {
        let (maybe_block, stream_tail) = stream.into_future().await;
        match maybe_block {
            Some(block) => {
                let res = process_network_block(
                    &blockchain,
                    block.clone(),
                    &mut watch_msg_box,
                    &mut get_next_block_scheduler,
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
                        tracing::info!(
                            reason = ?e,
                            "validation of an incoming block failed"
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
                Arc::clone(&new_block_ref),
                tip_update_mbox,
                network_msg_box,
            )
            .await?;
            Ok(())
        }
        None => Ok(()),
    }
}

async fn process_network_block(
    blockchain: &Blockchain,
    block: Block,
    watch_msg_box: &mut MessageBox<WatchMsg>,
    get_next_block_scheduler: &mut GetNextBlockScheduler,
) -> Result<Option<Arc<Ref>>, chain::Error> {
    let header = block.header().clone();
    let span = tracing::span!(
        Level::DEBUG,
        "network_block",
        block = %header.hash(),
        parent = %header.parent_id(),
        date = %header.block_date(),
    );

    async {
        get_next_block_scheduler
            .declare_completed(block.id())
            .unwrap_or_else(
                |e| tracing::error!(reason = ?e, "get next block schedule completion failed"),
            );
        let pre_checked = blockchain.pre_check_header(header, false).await?;
        match pre_checked {
            PreCheckedHeader::AlreadyPresent { .. } => {
                tracing::debug!("block is already present");
                Ok(None)
            }
            PreCheckedHeader::MissingParent { header } => {
                let parent_hash = header.parent_id();
                tracing::debug!("block is missing a locally stored parent");
                Err(Error::MissingParentBlock(parent_hash))
            }
            PreCheckedHeader::HeaderWithCache { parent_ref, .. } => {
                check_and_apply_block(blockchain, parent_ref, block, watch_msg_box).await
            }
        }
    }
    .instrument(span)
    .await
}

async fn check_and_apply_block(
    blockchain: &Blockchain,
    parent_ref: Arc<Ref>,
    block: Block,
    watch_msg_box: &mut MessageBox<WatchMsg>,
) -> Result<Option<Arc<Ref>>, chain::Error> {
    let post_checked = blockchain
        .post_check_header(
            block.header().clone(),
            parent_ref,
            CheckHeaderProof::Enabled,
        )
        .await?;
    tracing::debug!("applying block to storage");

    let block_for_watchers = block.clone();

    let applied_block = blockchain
        .apply_and_store_block(post_checked, block)
        .await?;
    if let AppliedBlock::New(block_ref) = applied_block {
        tracing::debug!("applied block to storage");

        watch_msg_box
            .try_send(WatchMsg::NewBlock(block_for_watchers))
            .unwrap_or_else(|err| {
                tracing::error!("cannot propagate block to watch clients: {}", err)
            });

        Ok(Some(block_ref))
    } else {
        tracing::debug!("block is already present in storage, not applied");
        Ok(None)
    }
}

async fn process_chain_headers(
    blockchain: Blockchain,
    handle: intercom::RequestStreamHandle<Header, ()>,
    mut pull_headers_scheduler: PullHeadersScheduler,
    mut network_msg_box: MessageBox<NetworkMsg>,
) {
    let (stream, reply) = handle.into_stream_and_reply();
    match candidate::advance_branch(blockchain, stream).await {
        Err(e) => {
            tracing::info!(
                reason = %e,
                "error processing an incoming header stream"
            );
            reply.reply_error(chain_header_error_into_reply(e));
        }
        Ok((header_ids, _maybe_remainder)) => {
            header_ids
                .iter()
                .try_for_each(|header_id| pull_headers_scheduler.declare_completed(*header_id))
                .unwrap_or_else(
                    |e| tracing::error!(reason = ?e, "get blocks schedule completion failed"),
                );

            if !header_ids.is_empty() {
                network_msg_box
                    .send(NetworkMsg::GetBlocks(header_ids))
                    .await
                    .map_err(|_| tracing::error!("cannot request blocks from network"))
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
    use super::chain::Error::*;

    match err {
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
