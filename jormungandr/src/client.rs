use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::network::p2p::{P2pTopology, Peer, PeersResponse};
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::HasHeader;

use futures::future::Either;
use tokio::prelude::*;
use tokio::timer::Timeout;

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
            let fut = handle.async_reply(get_block_tip(&task_data.blockchain_tip));
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
            let fut = handle.async_reply(get_headers(task_data.storage.clone(), ids));
            let logger = info.logger().new(o!("request" => "GetHeaders"));
            info.spawn(
                "GetHeaders",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS)).map_err(
                    move |e| {
                        warn!(
                            logger,
                            "request timed out or failed unexpectedly";
                            "error" => ?e,
                        );
                    },
                ),
            );
        }
        ClientMsg::GetHeadersRange(checkpoints, to, handle) => {
            let fut = handle_get_headers_range(task_data, checkpoints, to, handle);
            let logger = info.logger().new(o!("request" => "GetHeadersRange"));
            info.spawn(
                "GetHeadersRange",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_GET_HEADERS_RANGE)).map_err(
                    move |e| {
                        warn!(
                            logger,
                            "request timed out or failed unexpectedly";
                            "error" => ?e,
                        );
                    },
                ),
            );
        }
        ClientMsg::GetBlocks(ids, handle) => {
            let fut = handle.async_reply(get_blocks(task_data.storage.clone(), ids));
            let logger = info.logger().new(o!("request" => "GetBlocks"));
            info.spawn(
                "get blocks",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_GET_BLOCKS)).map_err(
                    move |e| {
                        warn!(
                            logger,
                            "request timed out or failed unexpectedly";
                            "error" => ?e,
                        );
                    },
                ),
            );
        }
        ClientMsg::PullBlocksToTip(from, handle) => {
            let fut = handle_pull_blocks_to_tip(task_data, from, handle);
            let logger = info.logger().new(o!("request" => "PullBlocksToTip"));
            info.spawn(
                "PullBlocksToTip",
                Timeout::new(fut, Duration::from_secs(PROCESS_TIMEOUT_PULL_BLOCKS_TO_TIP)).map_err(
                    move |e| {
                        warn!(
                            logger,
                            "request timed out or failed unexpectedly";
                            "error" => ?e,
                        );
                    },
                ),
            );
        }
    }
    Ok(())
}

fn get_block_tip(blockchain_tip: &Tip) -> impl Future<Item = Header, Error = Error> {
    blockchain_tip
        .get_ref()
        .and_then(|tip| Ok(tip.header().clone()))
}

fn get_peers(topology: &P2pTopology) -> impl Future<Item = PeersResponse, Error = Error> {
    // TODO: hardcoded for now but should come from some limit + client query
    topology.list_available_limit(192).and_then(|nodes| {
        let mut peers = Vec::new();
        for n in nodes.iter() {
            match n.address().and_then(|x| x.to_socketaddr()) {
                None => {}
                Some(addr) => peers.push(Peer { addr }),
            }
        }
        future::ok(PeersResponse { peers })
    })
}

fn handle_get_headers_range(
    task_data: &TaskData,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Header>,
) -> impl Future<Item = (), Error = ()> {
    let storage = task_data.storage.clone();
    storage
        .find_closest_ancestor(checkpoints, to)
        .then(move |res| match res {
            Ok(maybe_ancestor) => {
                let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
                let fut = storage
                    .send_branch(
                        to,
                        depth,
                        handle
                            .with(|res: Result<Block, Error>| Ok(res.map(|block| block.header()))),
                    )
                    .then(|_: Result<_, ReplySendError>| Ok(()));
                Either::A(fut)
            }
            Err(e) => Either::B(handle.async_error(e.into())),
        })
}

fn get_blocks(storage: Storage, ids: Vec<HeaderHash>) -> impl Stream<Item = Block, Error = Error> {
    stream::iter_ok(ids).and_then(move |id| {
        storage
            .get(id)
            .map_err(Into::into)
            .and_then(move |maybe_block| match maybe_block {
                Some(block) => Ok(block),
                None => Err(Error::not_found(format!(
                    "block {} is not known to this node",
                    id
                ))),
            })
    })
}

fn get_headers(
    storage: Storage,
    ids: Vec<HeaderHash>,
) -> impl Stream<Item = Header, Error = Error> {
    stream::iter_ok(ids).and_then(move |id| {
        storage
            .get(id)
            .map_err(Into::into)
            .and_then(move |maybe_block| match maybe_block {
                Some(block) => Ok(block.header()),
                None => Err(Error::not_found(format!(
                    "block {} is not known to this node",
                    id
                ))),
            })
    })
}

fn handle_pull_blocks_to_tip(
    task_data: &TaskData,
    checkpoints: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> impl Future<Item = (), Error = ()> {
    let storage = task_data.storage.clone();
    task_data
        .blockchain_tip
        .get_ref()
        .and_then(move |tip| {
            let tip_hash = tip.hash();
            storage
                .find_closest_ancestor(checkpoints, tip_hash)
                .map(move |maybe_ancestor| {
                    let depth = maybe_ancestor.map(|ancestor| ancestor.distance);
                    (storage, tip_hash, depth)
                })
        })
        .then(move |res| match res {
            Ok((storage, to, depth)) => {
                Either::A(storage.send_branch(to, depth, handle).then(|_| Ok(())))
            }
            Err(e) => Either::B(handle.async_error(e.into())),
        })
}
