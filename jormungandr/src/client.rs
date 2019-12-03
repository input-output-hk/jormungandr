use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::HasHeader;

use futures::future::{Either, FutureResult};
use tokio::prelude::*;

pub struct TaskData {
    pub storage: Storage,
    pub blockchain_tip: Tip,
}

enum TaskAction<GetBlockTip, GetHeaders, GetHeadersRange, GetBlocks, PullBlocksToTip> {
    Shutdown(FutureResult<(), ()>),
    GetBlockTip(GetBlockTip),
    GetHeaders(GetHeaders),
    GetHeadersRange(GetHeadersRange),
    GetBlocks(GetBlocks),
    PullBlocksToTip(PullBlocksToTip),
}

impl<
        GetBlockTip: Future<Item = (), Error = ()>,
        GetHeaders: Future<Item = (), Error = ()>,
        GetHeadersRange: Future<Item = (), Error = ()>,
        GetBlocks: Future<Item = (), Error = ()>,
        PullBlocksToTip: Future<Item = (), Error = ()>,
    > Future for TaskAction<GetBlockTip, GetHeaders, GetHeadersRange, GetBlocks, PullBlocksToTip>
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        use self::TaskAction::*;

        match self {
            Shutdown(fut) => fut.poll(),
            GetBlockTip(fut) => fut.poll(),
            GetHeaders(fut) => fut.poll(),
            GetHeadersRange(fut) => fut.poll(),
            GetBlocks(fut) => fut.poll(),
            PullBlocksToTip(fut) => fut.poll(),
        }
    }
}

pub fn handle_input(
    _info: &TokioServiceInfo,
    task_data: &mut TaskData,
    input: Input<ClientMsg>,
) -> impl Future<Item = (), Error = ()> {
    let cquery = match input {
        Input::Shutdown => return TaskAction::Shutdown(Ok(()).into()),
        Input::Input(msg) => msg,
    };

    match cquery {
        ClientMsg::GetBlockTip(handle) => {
            TaskAction::GetBlockTip(handle.async_reply(get_block_tip(&task_data.blockchain_tip)))
        }
        ClientMsg::GetHeaders(ids, handle) => {
            TaskAction::GetHeaders(handle.async_reply(get_headers(task_data.storage.clone(), ids)))
        }
        ClientMsg::GetHeadersRange(checkpoints, to, handle) => TaskAction::GetHeadersRange(
            handle_get_headers_range(task_data, checkpoints, to, handle),
        ),
        ClientMsg::GetBlocks(ids, handle) => {
            TaskAction::GetBlocks(handle.async_reply(get_blocks(task_data.storage.clone(), ids)))
        }
        ClientMsg::PullBlocksToTip(from, handle) => {
            TaskAction::PullBlocksToTip(handle_pull_blocks_to_tip(task_data, from, handle))
        }
    }
}

fn get_block_tip(blockchain_tip: &Tip) -> impl Future<Item = Header, Error = Error> {
    blockchain_tip
        .get_ref()
        .and_then(|tip| Ok(tip.header().clone()))
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
