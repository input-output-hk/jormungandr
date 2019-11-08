use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Storage, Tip};
use crate::intercom::{do_stream_reply, ClientMsg, Error, ReplyStreamHandle};
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::HasHeader;
use chain_storage::store;

use futures::future::Either;
use tokio::prelude::*;

pub struct TaskData {
    pub storage: Storage,
    pub block0_hash: HeaderHash,
    pub blockchain_tip: Tip,
}

pub fn handle_input(_info: &ThreadServiceInfo, task_data: &mut TaskData, input: Input<ClientMsg>) {
    let cquery = match input {
        Input::Shutdown => return,
        Input::Input(msg) => msg,
    };

    match cquery {
        ClientMsg::GetBlockTip(handler) => {
            handler.reply(handle_get_block_tip(&task_data.blockchain_tip))
        }
        ClientMsg::GetHeaders(ids, handler) => do_stream_reply(handler, |handler| {
            handle_get_headers(&task_data.storage, ids, handler)
        }),
        ClientMsg::GetHeadersRange(checkpoints, to, handler) => {
            do_stream_reply(handler, |handler| {
                handle_get_headers_range(&task_data.storage, checkpoints, to, handler)
            })
        }
        ClientMsg::GetBlocks(ids, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks(&task_data.storage, ids, handler)
        }),
        ClientMsg::GetBlocksRange(from, to, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks_range(&task_data.storage, from, to, handler)
        }),
        ClientMsg::PullBlocksToTip(from, handler) => do_stream_reply(handler, |handler| {
            handle_pull_blocks_to_tip(&task_data.storage, &task_data.blockchain_tip, from, handler)
        }),
    }
}

fn handle_get_block_tip(blockchain_tip: &Tip) -> Result<Header, Error> {
    let blockchain_tip = blockchain_tip.get_ref().wait().unwrap();

    Ok(blockchain_tip.header().clone())
}

const MAX_HEADERS: u64 = 2000;

fn handle_get_headers_range(
    storage: &Storage,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    let future = storage
        .find_closest_ancestor(checkpoints, to)
        .map_err(|e| e.into())
        .and_then(move |maybe_ancestor| match maybe_ancestor {
            Some(from) => Either::A(storage.stream_from_to(from, to).map_err(|e| e.into())),
            None => Either::B(future::err(Error::failed_precondition(
                "none of the checkpoints found in the local storage are ancestors \
                 of the requested end block",
            ))),
        })
        .and_then(move |stream| {
            // Send headers up to the maximum
            stream
                .map_err(|e| e.into())
                .take(MAX_HEADERS)
                .for_each(move |block| {
                    reply
                        .send(block.header())
                        .map_err(|_| Error::failed("failed to send reply"))?;
                    Ok(())
                })
        });

    future.wait()
}

fn handle_get_blocks_range(
    storage: &Storage,
    from: HeaderHash,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    // FIXME: remove double locking
    let storage = storage.get_inner().wait().unwrap();

    // FIXME: include the from block

    for x in store::iterate_range(&*storage, &from, &to)? {
        let info = x?;
        let (blk, _) = storage.get_block(&info.block_hash)?;
        if let Err(_) = reply.send(blk) {
            break;
        }
    }

    Ok(())
}

fn handle_get_blocks(
    storage: &Storage,
    ids: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    for id in ids.into_iter() {
        if let Some(blk) = storage.get(id).wait()? {
            if let Err(_) = reply.send(blk) {
                break;
            }
        } else {
            // TODO: reply this hash was not found?
        }
    }

    Ok(())
}

fn handle_get_headers(
    storage: &Storage,
    ids: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    for id in ids.into_iter() {
        if let Some(blk) = storage.get(id).wait()? {
            if let Err(_) = reply.send(blk.header()) {
                break;
            }
        } else {
            // TODO: reply this hash was not found?
        }
    }

    Ok(())
}

fn handle_pull_blocks_to_tip(
    storage: &Storage,
    blockchain_tip: &Tip,
    checkpoints: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    let tip = blockchain_tip.get_ref().wait().unwrap();
    let tip_hash = tip.hash();

    let future = storage
        .find_closest_ancestor(checkpoints, tip_hash)
        .map_err(|e| e.into())
        .and_then(move |maybe_ancestor| match maybe_ancestor {
            Some(from) => Either::A(storage.stream_from_to(from, tip_hash).map_err(|e| e.into())),
            None => Either::B(future::err(Error::failed_precondition(
                "none of the checkpoints found in the local storage \
                 are ancestors of the current tip",
            ))),
        })
        .and_then(move |stream| {
            // Send headers up to the maximum
            stream
                .map_err(|e| e.into())
                .take(MAX_HEADERS)
                .for_each(move |block| {
                    reply
                        .send(block)
                        .map_err(|_| Error::failed("failed to send reply"))?;
                    Ok(())
                })
        });

    future.wait()
}
