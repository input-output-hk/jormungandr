use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;
use crate::intercom::{ClientMsg, Error, ReplyHandle, ReplyStreamHandle};
use cardano_storage::iter;
use std::sync::mpsc::Receiver;

pub fn client_task<B: BlockConfig>(blockchain: BlockchainR<B>, r: Receiver<ClientMsg<B>>) {
    loop {
        let query = r.recv().unwrap();
        debug!("client query received: {:?}", query);

        match query {
            ClientMsg::GetBlockTip(mut handler) => handler.reply(handle_get_block_tip(&blockchain)),
            ClientMsg::GetBlockHeaders(checkpoints, to, mut handler) => {
                handler.reply(handle_get_block_headers(&blockchain, checkpoints, to))
            }
            ClientMsg::GetBlocks(from, to, handler) => {
                do_stream_reply(|| handle_get_blocks(&blockchain, from, to, handler))
            }
            ClientMsg::StreamBlocksToTip(from, handler) => {
                do_stream_reply(|| handle_stream_blocks_to_tip(&blockchain, from, handler))
            }
        }
    }
}

struct StreamReplyError<T>(Error, ReplyStreamHandle<T>);

fn do_stream_reply<T, F>(f: F)
where
    F: FnOnce() -> Result<ReplyStreamHandle<T>, StreamReplyError<T>>,
{
    let mut handler = match f() {
        Ok(handler) => handler,
        Err(StreamReplyError(e, mut handler)) => {
            handler.send_error(e.into());
            handler
        }
    };
    handler.close();
}

fn handle_get_block_tip<B: BlockConfig>(
    blockchain: &BlockchainR<B>,
) -> Result<B::BlockHeader, Error> {
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.get_tip();
    match blockchain.get_storage().read_block(tip.as_hash_bytes()) {
        Err(err) => Err(format!("Cannot read block '{}': {}", tip, err).into()),
        Ok(rblk) => {
            let blk = rblk.decode().unwrap();
            Ok(blk.get_header())
        }
    }
}

const MAX_HEADERS: usize = 2000;

fn handle_get_block_headers<B: BlockConfig>(
    blockchain: &BlockchainR<B>,
    checkpoints: Vec<B::BlockHash>,
    to: B::BlockHash,
) -> Result<Vec<B::Header>, Error> {
    let blockchain = blockchain.read().unwrap();

    /* Filter out the checkpoints that don't exist and sort them by
     * block date. */
    let mut checkpoints = checkpoints
        .iter()
        .filter_map(|checkpoint| {
            match blockchain
                .get_storage()
                .read_block(&checkpoint.as_hash_bytes())
            {
                Err(_) => None,
                Ok(rblk) => Some((
                    rblk.decode().unwrap().get_header().get_blockdate(),
                    checkpoint,
                )),
            }
        })
        .collect::<Vec<_>>();

    if !checkpoints.is_empty() {
        /* Start at the newest checkpoint. */
        checkpoints.sort_unstable();
        let from = checkpoints.last().unwrap().1;

        // FIXME: handle checkpoint == genesis

        /* Send headers up to the maximum. Skip the first block since
         * the range is exclusive of 'from'. */
        let mut skip = true;
        let mut headers = vec![];
        let mut err = None;
        for x in iter::Iter::new(&blockchain.get_storage(), from.clone(), to).unwrap() {
            match x {
                Err(err2) => {
                    err = Some(err2);
                }
                Ok((_, blk)) => {
                    if skip {
                        skip = false;
                    } else {
                        headers.push(blk.get_header());
                        if headers.len() >= MAX_HEADERS {
                            break;
                        }
                    }
                }
            };
        }

        match err {
            Some(err) => Err(Error::from_error(err)),
            None => Ok(headers),
        }
    } else {
        Ok(vec![])
    }
}

fn handle_get_blocks<B: BlockConfig>(
    blockchain: &BlockchainR<B>,
    from: B::BlockHash,
    to: B::BlockHash,
    mut reply: ReplyStreamHandle<B::Block>,
) -> Result<ReplyStreamHandle<B::Block>, StreamReplyError<B::Block>> {
    let blockchain = blockchain.read().unwrap();

    for x in iter::Iter::new(&blockchain.get_storage(), from, to).unwrap() {
        match x {
            Err(err) => return Err(StreamReplyError(Error::from_error(err), reply)),
            Ok((_rblk, blk)) => reply.send(blk), // FIXME: use rblk
        }
    }

    Ok(reply)
}

fn handle_stream_blocks_to_tip<B: BlockConfig>(
    blockchain: &BlockchainR<B>,
    mut from: Vec<B::BlockHash>,
    mut reply: ReplyStreamHandle<B::Block>,
) -> Result<ReplyStreamHandle<B::Block>, StreamReplyError<B::Block>> {
    let blockchain = blockchain.read().unwrap();

    // FIXME: handle multiple from addresses
    if from.len() != 1 {
        return Err(StreamReplyError(
            Error::from("only one checkpoint address is currently supported"),
            reply,
        ));
    }
    let from = from.remove(0);

    let tip = blockchain.get_tip();

    for x in iter::Iter::new(&blockchain.get_storage(), from, tip).unwrap() {
        match x {
            Err(err) => return Err(StreamReplyError(Error::from_error(err), reply)),
            Ok((_rblk, blk)) => reply.send(blk), // FIXME: use rblk
        }
    }

    Ok(reply)
}
