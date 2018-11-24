use super::{BlockHash};
use blockcfg::{Header, Block};
use blockchain::{BlockchainR};
use cardano_storage::{block_read, iter};
use intercom::*;
use std::sync::{mpsc::Receiver};

pub fn client_task(blockchain: BlockchainR, r: Receiver<ClientMsg>) {
    loop {
        let query = r.recv().unwrap();
        debug!("client query received: {:?}", query);

        match query {
            ClientMsg::GetBlockTip(mut handler) =>
                handler.reply(handle_get_block_tip(&blockchain)),
            ClientMsg::GetBlockHeaders(checkpoints, to, mut handler) =>
                handler.reply(handle_get_block_headers(&blockchain, checkpoints, to)),
            ClientMsg::GetBlocks(from, to, handler) =>
                handle_get_blocks(&blockchain, from, to, handler),
        }
    }
}

fn handle_get_block_tip(
    blockchain: &BlockchainR
) -> Result<Header, Error> {
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.get_tip();
    match block_read(blockchain.get_storage(), &tip) {
        None => {
            Err(format!("Cannot read block '{}'", tip).into())
        }
        Some(rblk) => {
            let blk = rblk.decode().unwrap();
            Ok(blk.get_header())
        }
    }
}

const MAX_HEADERS: usize = 2000;

fn handle_get_block_headers(
    blockchain: &BlockchainR,
    checkpoints: Vec<BlockHash>,
    to: BlockHash
) -> Result<Vec<Header>, Error> {
    let blockchain = blockchain.read().unwrap();

    /* Filter out the checkpoints that don't exist and sort them by
     * block date. */
    let mut checkpoints = checkpoints.iter().filter_map(
        |checkpoint|
        match block_read(blockchain.get_storage(), &checkpoint) {
            None => None,
            Some(rblk) => Some((rblk.decode().unwrap().get_header().get_blockdate(), checkpoint))
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
    blockchain: &BlockchainR,
    from: BlockHash,
    to: BlockHash,
    mut reply: Box<StreamReply<Block>>)
{
    let blockchain = blockchain.read().unwrap();

    for x in iter::Iter::new(&blockchain.get_storage(), from, to).unwrap() {
        match x {
            Err(err) => reply.send_error(Error::from_error(err)),
            Ok((_rblk, blk)) => reply.send(blk), // FIXME: use rblk
        }
    }

    reply.close();
}
