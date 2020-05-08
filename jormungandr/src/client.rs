use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::utils::async_msg::MessageQueue;
use crate::utils::task::TokioServiceInfo;
use chain_core::property::HasHeader;

use futures::prelude::*;
use tokio::time::timeout;

use std::time::Duration;

const PROCESS_TIMEOUT_GET_BLOCK_TIP: u64 = 5;
const PROCESS_TIMEOUT_GET_PEERS: u64 = 10;
const PROCESS_TIMEOUT_GET_HEADERS: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_HEADERS_RANGE: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_BLOCKS: u64 = 10 * 60;
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
            let logger = info.logger().new(o!("request" => "GetBlockTip"));
            info.spawn_fallible(
                "get block tip",
                timeout(Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCK_TIP), fut).map_err(
                    move |e| {
                        error!(
                            logger,
                            "request timed out or failed unexpectedly";
                            "error" => ?e,
                        );
                    },
                ),
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
    mut handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    let res = storage.find_closest_ancestor(checkpoints, to).await;
    match res {
        Ok(maybe_ancestor) => {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            storage
                .send_branch_with(to, depth, handle, |block| block.header())
                .await
        }
        Err(e) => handle.send(Err(e.into())).await,
    }
}

async fn handle_get_blocks(
    storage: Storage,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let mut handle = handle;
    for id in ids {
        let res = match storage.get(id).await {
            Ok(Some(block)) => Ok(block),
            Ok(None) => Err(Error::not_found(format!(
                "block {} is not known to this node",
                id
            ))),
            Err(e) => Err(e.into()),
        };
        handle.send(res).await?;
    }
    Ok(())
}

async fn handle_get_headers(
    storage: Storage,
    ids: Vec<HeaderHash>,
    mut handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    for id in ids {
        let res = match storage.get(id).await {
            Ok(Some(block)) => Ok(block.header()),
            Ok(None) => Err(Error::not_found(format!(
                "block {} is not known to this node",
                id
            ))),
            Err(e) => Err(e.into()),
        };
        handle.send(res).await?;
    }
    Ok(())
}

async fn handle_pull_blocks_to_tip(
    storage: Storage,
    blockchain_tip: Tip,
    checkpoints: Vec<HeaderHash>,
    mut handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let tip = blockchain_tip.get_ref().await;
    let tip_hash = tip.hash();
    let res = storage
        .find_closest_ancestor(checkpoints, tip_hash)
        .await
        .map(move |maybe_ancestor| {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            (tip_hash, depth)
        });
    match res {
        Ok((to, depth)) => storage.send_branch(to, depth, handle).await,
        Err(e) => handle.send(Err(e.into())).await,
    }
}
