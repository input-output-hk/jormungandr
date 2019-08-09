use crate::blockcfg::{Block, Header, HeaderHash};
use crate::blockchain::{Blockchain};
use crate::intercom::{do_stream_reply, ClientMsg, Error, ReplyStreamHandle};
use crate::utils::task::{Input, ThreadServiceInfo};
use chain_core::property::HasHeader;
use chain_storage::store;

pub fn handle_input(_info: &ThreadServiceInfo, blockchain: &Blockchain, input: Input<ClientMsg>) {
    let cquery = match input {
        Input::Shutdown => return,
        Input::Input(msg) => msg,
    };

    match cquery {
        ClientMsg::GetBlockTip(handler) => handler.reply(handle_get_block_tip(&blockchain)),
        ClientMsg::GetHeaders(ids, handler) => do_stream_reply(handler, |handler| {
            handle_get_headers(&blockchain, ids, handler)
        }),
        ClientMsg::GetHeadersRange(checkpoints, to, handler) => {
            do_stream_reply(handler, |handler| {
                handle_get_headers_range(&blockchain, checkpoints, to, handler)
            })
        }
        ClientMsg::GetBlocks(ids, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks(&blockchain, ids, handler)
        }),
        ClientMsg::GetBlocksRange(from, to, handler) => do_stream_reply(handler, |handler| {
            handle_get_blocks_range(&blockchain, from, to, handler)
        }),
        ClientMsg::PullBlocksToTip(from, handler) => do_stream_reply(handler, |handler| {
            handle_pull_blocks_to_tip(&blockchain, from, handler)
        }),
    }
}

fn handle_get_block_tip(blockchain: &BlockchainR) -> Result<Header, Error> {
    let blockchain = blockchain.lock_read();
    let tip = blockchain.get_tip().unwrap();
    let storage = blockchain.storage.read().unwrap();
    match storage.get_block(&tip) {
        Err(err) => Err(Error::failed(format!(
            "Cannot read block '{}': {}",
            tip, err
        ))),
        Ok((blk, _)) => Ok(blk.header()),
    }
}

const MAX_HEADERS: usize = 2000;

fn find_latest_checkpoint(
    blockchain: &Blockchain,
    checkpoints: &[HeaderHash],
) -> Option<HeaderHash> {
    // Filter out the checkpoints that don't exist
    // (or failed to be retrieved from the store for any other reason)
    // and find the latest by chain length.
    let storage = blockchain.storage.read().unwrap();
    checkpoints
        .iter()
        .filter_map(|hash| match storage.get_block_info(hash) {
            Ok(info) => Some((info.depth, hash)),
            Err(_) => None,
        })
        .max_by_key(|&(depth, _)| depth)
        .map(|(_, hash)| *hash)
}

fn handle_get_headers_range(
    blockchain: &Blockchain,
    checkpoints: Vec<HeaderHash>,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    let blockchain = blockchain.lock_read();

    let from = match find_latest_checkpoint(&blockchain, &checkpoints) {
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
    let storage = blockchain.storage.read().unwrap();
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
    blockchain: &Blockchain,
    from: HeaderHash,
    to: HeaderHash,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    // FIXME: remove double locking
    let blockchain = blockchain.lock_read();
    let storage = blockchain.storage.read().unwrap();

    // FIXME: include the from block

    for x in store::iterate_range(&*storage, &from, &to)? {
        let info = x?;
        let (blk, _) = storage.get_block(&info.block_hash)?;
        reply.send(blk);
    }

    Ok(())
}

fn handle_get_blocks(
    blockchain: &Blockchain,
    ids: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    let blockchain = blockchain.lock_read();

    for id in ids.into_iter() {
        let (blk, _) = blockchain.storage.read().unwrap().get_block(&id)?;
        reply.send(blk);
    }

    Ok(())
}

fn handle_get_headers(
    blockchain: &Blockchain,
    ids: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Header>,
) -> Result<(), Error> {
    let blockchain = blockchain.lock_read();

    for id in ids.into_iter() {
        let (blk, _) = blockchain.storage.read().unwrap().get_block(&id)?;
        reply.send(blk.header());
    }

    Ok(())
}

fn handle_pull_blocks_to_tip(
    blockchain: &Blockchain,
    checkpoints: Vec<HeaderHash>,
    reply: &mut ReplyStreamHandle<Block>,
) -> Result<(), Error> {
    let blockchain = blockchain.lock_read();

    let from = match find_latest_checkpoint(&blockchain, &checkpoints) {
        Some(hash) => hash,
        None => {
            return Err(Error::not_found(
                "none of the starting points are found in the blockchain",
            ))
        }
    };

    let tip = blockchain.get_tip().unwrap();

    let storage = blockchain.storage.read().unwrap();
    for x in store::iterate_range(&*storage, &from, &tip)? {
        let info = x?;
        let (blk, _) = blockchain
            .storage
            .read()
            .unwrap()
            .get_block(&info.block_hash)?;
        reply.send(blk);
    }

    Ok(())
}
