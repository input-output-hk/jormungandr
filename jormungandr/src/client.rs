use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{ClientMsg, Error, ReplySendError, ReplyStreamHandle};
use crate::utils::task::{Input, TokioServiceInfo};
use chain_core::property::HasHeader;

use futures::future::{Either, FutureResult};
use tokio::prelude::*;

pub struct TaskData {
    pub storage: Storage,
    pub block0_hash: HeaderHash,
    pub blockchain_tip: Tip,
}

enum TaskAction<
    GetBlockTip,
    GetHeaders,
    GetHeadersRange,
    GetBlocks,
    GetBlocksRange,
    PullBlocksToTip,
> {
    Shutdown(FutureResult<(), ()>),
    GetBlockTip(GetBlockTip),
    GetHeaders(GetHeaders),
    GetHeadersRange(GetHeadersRange),
    GetBlocks(GetBlocks),
    GetBlocksRange(GetBlocksRange),
    PullBlocksToTip(PullBlocksToTip),
}

impl<
        GetBlockTip: Future<Item = (), Error = ()>,
        GetHeaders: Future<Item = (), Error = ()>,
        GetHeadersRange: Future<Item = (), Error = ()>,
        GetBlocks: Future<Item = (), Error = ()>,
        GetBlocksRange: Future<Item = (), Error = ()>,
        PullBlocksToTip: Future<Item = (), Error = ()>,
    > Future
    for TaskAction<
        GetBlockTip,
        GetHeaders,
        GetHeadersRange,
        GetBlocks,
        GetBlocksRange,
        PullBlocksToTip,
    >
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
            GetBlocksRange(fut) => fut.poll(),
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
            handle_get_headers_range(task_data.storage.clone(), checkpoints, to, handle),
        ),
        ClientMsg::GetBlocks(ids, handle) => {
            TaskAction::GetBlocks(handle.async_reply(get_blocks(task_data.storage.clone(), ids)))
        }
        ClientMsg::GetBlocksRange(from, to, handle) => TaskAction::GetBlocksRange(
            handle_get_blocks_range(&task_data.storage, from, to, handle),
        ),
        ClientMsg::PullBlocksToTip(from, handle) => {
            TaskAction::PullBlocksToTip(handle_pull_blocks_to_tip(
                task_data.storage.clone(),
                task_data.blockchain_tip.clone(),
                from,
                handle,
            ))
        }
    }
}

fn get_block_tip(blockchain_tip: &Tip) -> impl Future<Item = Header, Error = Error> {
    blockchain_tip
        .get_ref()
        .and_then(|tip| Ok(tip.header().clone()))
}

fn handle_get_headers_range(
    storage: Storage,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    handle: ReplyStreamHandle<Header>,
) -> impl Future<Item = (), Error = ()> {
    storage
        .find_closest_ancestor(checkpoints, to)
        .then(move |res| match res {
            Ok(Some(from)) => {
                let fut = storage
                    .send_from_to(
                        from,
                        to,
                        handle.with(|res: Result<Block, _>| Ok(res.map(|block| block.header()))),
                    )
                    .map_err(|_: ReplySendError| ());
                Either::A(fut)
            }
            Ok(None) => Either::B(handle.async_error(Error::not_found(
                "none of the checkpoints found in the local storage \
                 are ancestors of the requested end block",
            ))),
            Err(e) => Either::B(handle.async_error(e.into())),
        })
}

fn handle_get_blocks_range(
    storage: &Storage,
    from: HeaderHash,
    to: HeaderHash,
    handle: ReplyStreamHandle<Block>,
) -> impl Future<Item = (), Error = ()> {
    storage.send_from_to(from, to, handle).map_err(|_| ())
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
    storage: Storage,
    blockchain_tip: Tip,
    checkpoints: Vec<HeaderHash>,
    handle: ReplyStreamHandle<Block>,
) -> impl Future<Item = (), Error = ()> {
    blockchain_tip
        .get_ref()
        .and_then(move |tip| {
            let tip_hash = tip.hash();
            storage
                .find_closest_ancestor(checkpoints, tip_hash)
                .map(move |maybe_ancestor| (storage, maybe_ancestor, tip_hash))
        })
        .then(move |res| match res {
            Ok((storage, Some(from), to)) => {
                Either::A(storage.send_from_to(from, to, handle).map_err(|_| ()))
            }
            Ok((_, None, _)) => Either::B(handle.async_error(Error::not_found(
                "none of the checkpoints found in the local storage \
                 are ancestors of the current tip",
            ))),
            Err(e) => Either::B(handle.async_error(e.into())),
        })
}
