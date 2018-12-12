use blockcfg::{chain, BlockConfig, Cardano};
use blockchain::{BlockchainR};
use cardano_storage::{block_read, iter};
use intercom::*;
use std::sync::{mpsc::Receiver};

pub fn client_task(blockchain: BlockchainR<Cardano>, r: Receiver<ClientMsg<Cardano>>) {
    loop {
        let query = r.recv().unwrap();
        debug!("client query received: {:?}", query);

        match query {
            ClientMsg::GetBlockTip(mut handler) =>
                handler.reply(handle_get_block_tip(&blockchain)),
            ClientMsg::GetBlockHeaders(checkpoints, to, mut handler) =>
                handler.reply(handle_get_block_headers(&blockchain, checkpoints, to)),
            ClientMsg::GetBlocks(from, to, handler) =>
                do_stream_reply(
                    || handle_get_blocks(&blockchain, from, to, handler),
                ),
            ClientMsg::StreamBlocksToTip(from, handler) =>
                do_stream_reply(
                    || handle_stream_blocks_to_tip(&blockchain, from, handler),
                ),
        }
    }
}

struct StreamReplyError<T>(Error, BoxStreamReply<T>);

fn do_stream_reply<T, F>(f: F)
where
    F: FnOnce() -> Result<BoxStreamReply<T>, StreamReplyError<T>>,
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

fn handle_get_block_tip(
    blockchain: &BlockchainR<Cardano>
) -> Result<chain::cardano::Header, Error> {
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.get_tip();
    match block_read(blockchain.get_storage(), &tip) {
        Err(err) => {
            Err(format!("Cannot read block '{}': {}", tip, err).into())
        }
        Ok(rblk) => {
            let blk = rblk.decode().unwrap();
            Ok(blk.get_header())
        }
    }
}

const MAX_HEADERS: usize = 2000;

fn handle_get_block_headers(
    blockchain: &BlockchainR<Cardano>,
    checkpoints: Vec<chain::cardano::BlockHash>,
    to: chain::cardano::BlockHash
) -> Result<Vec<chain::cardano::Header>, Error> {
    let blockchain = blockchain.read().unwrap();

    /* Filter out the checkpoints that don't exist and sort them by
     * block date. */
    let mut checkpoints = checkpoints.iter().filter_map(
        |checkpoint|
        match block_read(blockchain.get_storage(), &checkpoint) {
            Err(err) => None,
            Ok(rblk) => Some((rblk.decode().unwrap().get_header().get_blockdate(), checkpoint))
        }
    ).collect::<Vec<_>>();

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
                Err(err2) => { err = Some(err2); },
                Ok((rblk, blk)) => {
                    if skip {
                        skip = false;
                    } else {
                        headers.push(blk.get_header());
                        if headers.len() >= MAX_HEADERS { break; }
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

fn handle_get_blocks(
    blockchain: &BlockchainR<Cardano>,
    from: chain::cardano::BlockHash,
    to: chain::cardano::BlockHash,
    mut reply: BoxStreamReply<chain::cardano::Block>
) -> Result<BoxStreamReply<chain::cardano::Block>, StreamReplyError<chain::cardano::Block>> {
    let blockchain = blockchain.read().unwrap();

    for x in iter::Iter::new(&blockchain.get_storage(), from, to).unwrap() {
        match x {
            Err(err) => return Err(StreamReplyError(Error::from_error(err), reply)),
            Ok((_rblk, blk)) => reply.send(blk), // FIXME: use rblk
        }
    }

    Ok(reply)
}

fn handle_stream_blocks_to_tip(
    blockchain: &BlockchainR<Cardano>,
    mut from: Vec<chain::cardano::BlockHash>,
    mut reply: BoxStreamReply<chain::cardano::Block>,
) -> Result<BoxStreamReply<chain::cardano::Block>, StreamReplyError<chain::cardano::Block>> {
    let blockchain = blockchain.read().unwrap();

    // FIXME: handle multiple from addresses
    if from.len() != 1 {
        return Err(
            StreamReplyError(
                Error::from("only one checkpoint address is currently supported"),
                reply,
            )
        );
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
