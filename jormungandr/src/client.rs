use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Branch, Storage};
use crate::intercom::{do_stream_reply, ClientMsg, Error, ReplyStreamHandle};
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::HasHeader;
use chain_storage::store;
use tokio::prelude::*;

pub struct TaskData {
    pub storage: Storage,
    pub block0_hash: HeaderHash,
    pub blockchain_tip: Branch,
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
                handle_get_headers_range(
                    &task_data.storage,
                    &task_data.block0_hash,
                    checkpoints,
                    to,
                    handler,
                )
            })
        }
        ClientMsg::GetBlocks(ids, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks(&task_data.storage, ids, handler)
        }),
        ClientMsg::GetBlocksRange(from, to, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks_range(&task_data.storage, from, to, handler)
        }),
        ClientMsg::PullBlocksToTip(from, handler) => do_stream_reply(handler, |handler| {
            handle_pull_blocks_to_tip(
                &task_data.storage,
                &task_data.block0_hash,
                &task_data.blockchain_tip,
                from,
                handler,
            )
        }),
    }
}

fn handle_get_block_tip(blockchain_tip: &Branch) -> Result<Header, Error> {
    let blockchain_tip = blockchain_tip.get_ref().wait().unwrap();

    Ok(blockchain_tip.header().clone())
}

const MAX_HEADERS: usize = 2000;

fn find_latest_checkpoint(
    checkpoints: &[HeaderHash],
    storage: &Storage,
    block0_hash: &HeaderHash,
) -> Result<HeaderHash, Error> {
    // Filter out the checkpoints that don't exist in the storage;
    // among the checkpoints present, find the latest by chain length.
    let mut latest_checkpoint = None;
    for hash in checkpoints {
        match storage.get_with_info(hash.clone()).wait() {
            Ok(Some((_, info))) => match latest_checkpoint {
                None => {
                    latest_checkpoint = Some((info.depth, hash));
                }
                Some((latest_depth, _)) => {
                    if info.depth > latest_depth {
                        latest_checkpoint = Some((info.depth, hash));
                    }
                }
            },
            Ok(None) => continue,
            Err(e) => return Err(e.into()),
        }
    }
    match latest_checkpoint {
        Some((_, hash)) => Ok(hash.clone()),
        None => Ok(block0_hash.clone()),
    }
}

fn handle_get_headers_range(
    storage: &Storage,
    block0_hash: &HeaderHash,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    let from = find_latest_checkpoint(&checkpoints, storage, block0_hash)?;

    /* Send headers up to the maximum. */
    let mut header_count = 0usize;
    let storage = storage.get_inner().wait().unwrap();
    for x in store::iterate_range(&*storage, &from, &to)? {
        match x {
            Err(err) => return Err(Error::from(err)),
            Ok(info) => {
                let (block, _) = storage.get_block(&info.block_hash)?;
                reply.send(block.header());
                header_count += 1;
                if header_count >= MAX_HEADERS {
                    break;
                }
            }
        };
    }

    Ok(())
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
        reply.send(blk);
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
            reply.send(blk);
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
            reply.send(blk.header());
        } else {
            // TODO: reply this hash was not found?
        }
    }

    Ok(())
}

fn handle_pull_blocks_to_tip(
    storage: &Storage,
    block0_hash: &HeaderHash,
    blockchain_tip: &Branch,
    checkpoints: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    let from = find_latest_checkpoint(&checkpoints, &storage, &block0_hash)?;

    let tip = blockchain_tip.get_ref().wait().unwrap();

    let storage = storage.get_inner().wait().unwrap();
    for x in store::iterate_range(&*storage, &from, &tip.hash())? {
        let info = x?;
        let (blk, _) = storage.get_block(&info.block_hash)?;
        reply.send(blk);
    }

    Ok(())
}
