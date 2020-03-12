use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::network::p2p::{P2pTopology, Peer, PeersResponse};
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::HasHeader;

use futures03::compat::*;
use futures03::sink::SinkExt;
use tokio::prelude::*;
use tokio::timer::Timeout;

use std::time::Duration;

const PROCESS_TIMEOUT_GET_BLOCK_TIP: u64 = 5;
const PROCESS_TIMEOUT_GET_PEERS: u64 = 10;
const PROCESS_TIMEOUT_GET_HEADERS: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_HEADERS_RANGE: u64 = 5 * 60;
const PROCESS_TIMEOUT_GET_BLOCKS: u64 = 10 * 60;
const PROCESS_TIMEOUT_PULL_BLOCKS_TO_TIP: u64 = 60 * 60;

#[derive(Clone)]
pub struct TaskData {
    pub storage: Storage,
    pub blockchain_tip: Tip,
    pub topology: P2pTopology,
}

pub fn handle_input(
    info: &TokioServiceInfo,
    task_data: &mut TaskData,
    input: Input<ClientMsg>,
) -> Result<(), ()> {
    let cquery = match input {
        Input::Shutdown => return Ok(()),
        Input::Input(msg) => msg,
    };

    match cquery {
        ClientMsg::GetBlockTip(handle) => {
            let tip = task_data.blockchain_tip.clone();
            let fut = handle.async_reply(Compat::new(Box::pin(async move {
                Ok(get_block_tip(tip).await)
            })));
            let logger = info.logger().new(o!("request" => "GetBlockTip"));
            info.spawn(
                "get block tip",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCK_TIP)).map_err(
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
        ClientMsg::GetPeers(handle) => {
            let fut = handle.async_reply(get_peers(&task_data.topology));
            let logger = info.logger().new(o!("request" => "GetPeers"));

            info.spawn(
                "get peers",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_GET_PEERS)).map_err(
                    move |e| {
                        error!(
                            logger,
                            "request timed out of failed unexpectdly";
                            "error" => ?e,
                        );
                    },
                ),
            );
        }
        ClientMsg::GetHeaders(ids, handle) => {
            info.timeout_spawn_failable_std(
                "GetHeaders",
                Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS),
                handle_get_headers(task_data.clone(), ids, handle),
            );
        }
        ClientMsg::GetHeadersRange(checkpoints, to, handle) => {
            info.timeout_spawn_std(
                "GetHeadersRange",
                Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS_RANGE),
                handle_get_headers_range(task_data.clone(), checkpoints, to, handle),
            );
        }
        ClientMsg::GetBlocks(ids, handle) => {
            info.timeout_spawn_failable_std(
                "get blocks",
                Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCKS),
                handle_get_blocks(task_data.clone(), ids, handle),
            );
        }
        ClientMsg::PullBlocksToTip(from, handle) => {
            let fut = handle_pull_blocks_to_tip(task_data.clone(), from, handle);
            info.timeout_spawn_std(
                "PullBlocksToTip",
                Duration::from_secs(PROCESS_TIMEOUT_PULL_BLOCKS_TO_TIP),
                fut,
            );
        }
    }
    Ok(())
}

async fn get_block_tip(blockchain_tip: Tip) -> Header {
    let tip = blockchain_tip.get_ref().await;
    tip.header().clone()
}

fn get_peers(topology: &P2pTopology) -> impl Future<Item = PeersResponse, Error = Error> {
    topology.view(poldercast::Selection::Any).and_then(|view| {
        let mut peers = Vec::new();
        for n in view.peers.into_iter() {
            if let Some(addr) = n.address() {
                peers.push(Peer { addr });
            }
        }
        if peers.len() == 0 {
            // No peers yet, put self as the peer to bootstrap from
            if let Some(addr) = view.self_node.address().and_then(|x| x.to_socketaddr()) {
                peers.push(Peer { addr });
            }
        }
        future::ok(PeersResponse { peers })
    })
}

async fn handle_get_headers_range(
    task_data: TaskData,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Header>,
) {
    let res = task_data
        .storage
        .find_closest_ancestor(checkpoints, to)
        .await;
    match res {
        Ok(maybe_ancestor) => {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            let _ = task_data
                .storage
                .send_branch_with(to, depth, handle.into_03(), |block| block.header())
                .await;
        }
        Err(e) => handle.async_error(e.into()).compat().await.unwrap(),
    }
}

async fn handle_get_blocks(
    task_data: TaskData,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> Result<(), ReplySendError> {
    let mut handle = handle.into_03();
    for id in ids {
        let res = match task_data.storage.get(id).await {
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
    task_data: TaskData,
    ids: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Header>,
) -> Result<(), ReplySendError> {
    let mut handle = handle.into_03();
    for id in ids {
        let res = match task_data.storage.get(id).await {
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
    task_data: TaskData,
    checkpoints: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) {
    let tip = task_data.blockchain_tip.get_ref().await;
    let tip_hash = tip.hash();
    let res = task_data
        .storage
        .find_closest_ancestor(checkpoints, tip_hash)
        .await
        .map(move |maybe_ancestor| {
            let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
            (task_data.storage, tip_hash, depth)
        });
    match res {
        Ok((storage, to, depth)) => {
            let _ = storage.send_branch(to, depth, handle.into_03()).await;
        }
        Err(e) => {
            let _ = handle.async_error(e.into()).compat().await;
        }
    }
}
