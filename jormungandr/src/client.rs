use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Branch, Storage};
use crate::intercom::{do_stream_reply, ClientMsg, Error, ReplyStreamHandle};
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::HasHeader;
use chain_storage::store;
use tokio::prelude::*;

pub fn handle_input(
    _info: &ThreadServiceInfo,
    storage: &Storage,
    blockchain_tip: &Branch,
    input: Input<ClientMsg>,
) {
    let cquery = match input {
        Input::Shutdown => return,
        Input::Input(msg) => msg,
    };

    match cquery {
        ClientMsg::GetBlockTip(handler) => handler.reply(handle_get_block_tip(blockchain_tip)),
        ClientMsg::GetHeaders(ids, handler) => {
            do_stream_reply(handler, |handler| handle_get_headers(storage, ids, handler))
        }
        ClientMsg::GetHeadersRange(checkpoints, to, handler) => {
            do_stream_reply(handler, |handler| {
                handle_get_headers_range(storage, checkpoints, to, handler)
            })
        }
        ClientMsg::GetBlocks(ids, handler) => {
            do_stream_reply(handler, |handler| handle_get_blocks(storage, ids, handler))
        }
        ClientMsg::GetBlocksRange(from, to, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks_range(storage, from, to, handler)
        }),
        ClientMsg::PullBlocksToTip(from, handler) => do_stream_reply(handler, |handler| {
            handle_pull_blocks_to_tip(storage, blockchain_tip, from, handler)
        }),
    }
}

fn handle_get_block_tip(blockchain_tip: &Branch) -> Result<Header, Error> {
    let blockchain_tip = blockchain_tip.get_ref().wait().unwrap();

    Ok(blockchain_tip.header().clone())
}

const MAX_HEADERS: usize = 2000;

fn find_latest_checkpoint(storage: &Storage, checkpoints: &[HeaderHash]) -> Option<HeaderHash> {
    // Filter out the checkpoints that don't exist
    // (or failed to be retrieved from the store for any other reason)
    // and find the latest by chain length.
    checkpoints
        .iter()
        .filter_map(|hash| match storage.get_with_info(hash.clone()).wait() {
            Ok(Some((_, info))) => Some((info.depth, hash)),
            Ok(None) => None,
            Err(_) => None,
        })
        .max_by_key(|&(depth, _)| depth)
        .map(|(_, hash)| *hash)
}

fn handle_get_headers_range(
    storage: &Storage,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    let from = match find_latest_checkpoint(&storage, &checkpoints) {
        Some(hash) => hash,
        None => {
            return Err(Error::not_found(
                "none of the starting points are found in the blockchain",
            ))
        }
    };

    // FIXME: handle checkpoint == genesis

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
    blockchain_tip: &Branch,
    checkpoints: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    let from = match find_latest_checkpoint(storage, &checkpoints) {
        Some(hash) => hash,
        None => {
            return Err(Error::not_found(
                "none of the starting points are found in the blockchain",
            ))
        }
    };

    let tip = blockchain_tip.get_ref().wait().unwrap();

    let storage = storage.get_inner().wait().unwrap();
    for x in store::iterate_range(&*storage, &from, tip.hash())? {
        let info = x?;
        let (blk, _) = storage.get_block(&info.block_hash)?;
        reply.send(blk);
    }

    Ok(())
}
