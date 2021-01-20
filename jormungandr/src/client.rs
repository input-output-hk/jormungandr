use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::utils::async_msg::MessageQueue;
use crate::utils::task::TokioServiceInfo;
use chain_core::property::HasHeader;

use futures::prelude::*;
use tokio::time::timeout;
use tracing::{span, Level};
use tracing_futures::Instrument;

use std::time::Duration;

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
        ClientMsg::GetHeadersRange(checkpoints, to, handle) => {
            let storage = task_data.storage.clone();
            info.timeout_spawn_fallible(
                "GetHeadersRange",
                Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS_RANGE),
                handle_get_headers_range(storage, checkpoints, to, handle),
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

async fn handle_get_headers_range(
    storage: Storage,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    let res = storage.find_closest_ancestor(checkpoints, to);
    match res {
        Ok(maybe_ancestor) => {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            storage
                .send_branch_with(to, depth, handle, |block| block.header())
                .await
        }
        Err(e) => {
            handle.reply_error(e.into());
            Ok(())
        }
    }
}

async fn handle_get_blocks(
    storage: Storage,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let mut sink = handle.start_sending();
    for id in ids {
        let res = match storage.get(id) {
            Ok(Some(block)) => Ok(block),
            Ok(None) => Err(Error::not_found(format!(
                "block {} is not known to this node",
                id
            ))),
            Err(e) => Err(e.into()),
        };
        let is_err = res.is_err();
        sink.send(res).await?;
        if is_err {
            break;
        }
    }
    sink.close().await
}

async fn handle_get_headers(
    storage: Storage,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    let mut sink = handle.start_sending();
    for id in ids {
        let res = match storage.get(id) {
            Ok(Some(block)) => Ok(block.header()),
            Ok(None) => Err(Error::not_found(format!(
                "block {} is not known to this node",
                id
            ))),
            Err(e) => Err(e.into()),
        };
        let is_err = res.is_err();
        sink.send(res).await?;
        if is_err {
            break;
        }
    }
    sink.close().await
}

async fn handle_pull_blocks(
    storage: Storage,
    from: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    use crate::intercom::Error as IntercomError;

    let res = storage
        .find_closest_ancestor(from, to)
        .map_err(Into::into)
        .and_then(move |maybe_ancestor| {
            maybe_ancestor
                .map(|ancestor| (to, ancestor.distance))
                .ok_or_else(|| IntercomError::not_found("`from` not found"))
        });
    match res {
        Ok((to, depth)) => storage.send_branch(to, Some(depth), handle).await,
        Err(e) => {
            handle.reply_error(e);
            Ok(())
        }
    }
}

async fn handle_pull_blocks_to_tip(
    storage: Storage,
    blockchain_tip: Tip,
    checkpoints: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let tip = blockchain_tip.get_ref().await;
    let tip_hash = tip.hash();
    let res = storage
        .find_closest_ancestor(checkpoints, tip_hash)
        .map(move |maybe_ancestor| {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            (tip_hash, depth)
        });
    match res {
        Ok((to, depth)) => storage.send_branch(to, depth, handle).await,
        Err(e) => {
            handle.reply_error(e.into());
            Ok(())
        }
    }
}
