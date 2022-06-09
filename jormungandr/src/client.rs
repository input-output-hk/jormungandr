use crate::{
    blockcfg::{Block, Header, HeaderHash},
    blockchain::{Storage, Tip},
    intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle},
    utils::{async_msg::MessageQueue, task::TokioServiceInfo},
};
use futures::prelude::*;
use std::{convert::identity, time::Duration};
use tokio::time::timeout;
use tracing::{span, Level};
use tracing_futures::Instrument;

const PROCESS_TIMEOUT_GET_BLOCK_TIP: u64 = 5;
#[allow(dead_code)]
const PROCESS_TIMEOUT_GET_PEERS: u64 = 10;
const PROCESS_TIMEOUT_GET_HEADERS: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_HEADERS_RANGE: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_BLOCKS: u64 = 10 * 60;
const PROCESS_TIMEOUT_PULL_BLOCKS: u64 = 60 * 60;
const PROCESS_TIMEOUT_PULL_BLOCKS_TO_TIP: u64 = 60 * 60;

pub struct TaskData {
    pub storage: Storage,
    pub blockchain_tip: Tip,
}

pub async fn start(
    info: TokioServiceInfo,
    mut task_data: TaskData,
    mut input: MessageQueue<ClientMsg>,
) {
    while let Some(input) = input.next().await {
        handle_input(&info, &mut task_data, input);
    }
}

fn handle_input(info: &TokioServiceInfo, task_data: &mut TaskData, input: ClientMsg) {
    match input {
        ClientMsg::GetBlockTip(handle) => {
            let blockchain_tip = task_data.blockchain_tip.clone();
            let fut = async move {
                let tip = get_block_tip(blockchain_tip).await;
                handle.reply_ok(tip);
            };
            let span =
                span!(parent: info.span(), Level::TRACE, "sub_task", request = "GetBlockTip");

            info.spawn_fallible(
                "get block tip",
                timeout(Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCK_TIP), fut)
                    .map_err(|e| {
                        tracing::error!(
                            error = ?e,
                            "request timed out or failed unexpectedly"
                        );
                    })
                    .instrument(span),
            );
        }
        ClientMsg::GetHeaders(ids, handle) => {
            let storage = task_data.storage.clone();
            info.timeout_spawn_fallible(
                "GetHeaders",
                Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS),
                handle_get_headers(storage, ids, handle),
            );
        }
        ClientMsg::PullHeaders(from, to, handle) => {
            let storage = task_data.storage.clone();
            info.timeout_spawn_fallible(
                "PullHeaders",
                Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS_RANGE),
                handle_get_headers_range(storage, from, to, handle),
            );
        }
        ClientMsg::GetBlocks(ids, handle) => {
            let storage = task_data.storage.clone();
            info.timeout_spawn_fallible(
                "get blocks",
                Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCKS),
                handle_get_blocks(storage, ids, handle),
            );
        }
        ClientMsg::PullBlocks(from, to, handle) => {
            let storage = task_data.storage.clone();
            info.timeout_spawn_fallible(
                "PullBlocks",
                Duration::from_secs(PROCESS_TIMEOUT_PULL_BLOCKS),
                handle_pull_blocks(storage, from, to, handle),
            );
        }
        ClientMsg::PullBlocksToTip(from, handle) => {
            let storage = task_data.storage.clone();
            let blockchain_tip = task_data.blockchain_tip.clone();
            info.timeout_spawn_fallible(
                "PullBlocksToTip",
                Duration::from_secs(PROCESS_TIMEOUT_PULL_BLOCKS_TO_TIP),
                handle_pull_blocks_to_tip(storage, blockchain_tip, from, handle),
            );
        }
    }
}

async fn get_block_tip(blockchain_tip: Tip) -> Header {
    let tip = blockchain_tip.get_ref().await;
    tip.header().clone()
}

fn get_block_from_storage(storage: &Storage, id: HeaderHash) -> Result<Block, Error> {
    match storage.get(id) {
        Ok(Some(block)) => Ok(block),
        Ok(None) => Err(Error::not_found(format!(
            "block {} is not known to this node",
            id
        ))),
        Err(e) => Err(e.into()),
    }
}

// Stop after sending the first Err() variant
//
// Common base for GetBlocks and GetHeaders
async fn fuse_send_items<T, V>(
    items: T,
    reply_handle: ReplyStreamHandle<V>,
) -> Result<(), ReplySendError>
where
    T: IntoIterator<Item = Result<V, Error>>,
{
    let mut sink = reply_handle.start_sending();
    for item in items.into_iter() {
        let err = item.is_err();
        sink.feed(item).await?;
        if err {
            break;
        }
    }
    sink.close().await
}

// Send a range of blocks info directly from the storage to the stream.
// The starting point is determined by the closest ancestor of 'to'
// among the blocks specified in 'from'.
// The transformation function is applied to the block contents before
// sending it.
//
// Commong behavior for PullHeaders, PullBlocks, PullBlocksToTip
async fn send_range_from_storage<T, F>(
    storage: Storage,
    from: Vec<HeaderHash>,
    to: HeaderHash,
    f: F,
    handle: ReplyStreamHandle<T>,
) -> Result<(), ReplySendError>
where
    F: FnMut(Block) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let closest_ancestor = storage
        .find_closest_ancestor(from, to)
        .map_err(Into::into)
        .and_then(move |maybe_ancestor| {
            maybe_ancestor
                .map(|ancestor| (to, ancestor.distance))
                .ok_or_else(|| Error::not_found("Could not find a known block in `from`"))
        });
    match closest_ancestor {
        Ok((to, depth)) => storage.send_branch_with(to, Some(depth), handle, f).await,
        Err(e) => {
            handle.reply_error(e);
            Ok(())
        }
    }
}

async fn handle_get_blocks(
    storage: Storage,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    fuse_send_items(
        ids.into_iter()
            .map(|id| get_block_from_storage(&storage, id)),
        handle,
    )
    .await
}

async fn handle_get_headers(
    storage: Storage,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    fuse_send_items(
        ids.into_iter()
            .map(|id| get_block_from_storage(&storage, id).map(|block| block.header().clone())),
        handle,
    )
    .await
}

async fn handle_get_headers_range(
    storage: Storage,
    from: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    send_range_from_storage(storage, from, to, |block| block.header().clone(), handle).await
}

async fn handle_pull_blocks(
    storage: Storage,
    from: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    send_range_from_storage(storage, from, to, identity, handle).await
}

async fn handle_pull_blocks_to_tip(
    storage: Storage,
    blockchain_tip: Tip,
    from: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let tip = get_block_tip(blockchain_tip).await.id();
    send_range_from_storage(storage, from, tip, identity, handle).await
}
