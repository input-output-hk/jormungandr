use super::{BlockHash};
use blockchain::{BlockchainR};
use cardano_storage::{block_read, iter};
use intercom::*;
use protocol::{Message, protocol::{Response, BlockHeaders}};
use std::sync::{mpsc::Receiver};

pub fn client_task(blockchain: BlockchainR, r: Receiver<ClientMsg>) {
    loop {
        let query = r.recv().unwrap();
        debug!("client query received: {:?}", query);

        match query {
            ClientMsg::GetBlockTip(handler) =>
                handle_get_block_tip(&blockchain, handler),
            ClientMsg::GetBlockHeaders(checkpoints, to, handler) =>
                handle_get_block_headers(&blockchain, checkpoints, to, handler),
            ClientMsg::GetBlocks(from, to, handler) =>
                handle_get_blocks(&blockchain, from, to, handler),
            _ => unimplemented!()
        }
    }
}

fn handle_get_block_tip(
    blockchain: &BlockchainR,
    handler: NetworkHandler<ClientMsgGetHeaders>)
{
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.get_tip().0;
    let resp = match block_read(blockchain.get_storage(), &tip) {
        None => Response::Err(format!("Cannot read block '{}'", tip)),
        Some(rblk) => {
            let blk = rblk.decode().unwrap();
            Response::Ok(BlockHeaders(vec![blk.get_header()]))
        }
    };
    handler.sink.unbounded_send(
        Message::BlockHeaders(handler.identifier, resp)).unwrap();
    handler.sink.unbounded_send(
        Message::CloseConnection(handler.identifier)).unwrap();
}

const MAX_HEADERS: usize = 2000;

fn handle_get_block_headers(
    blockchain: &BlockchainR,
    checkpoints: Vec<BlockHash>,
    to: BlockHash,
    handler: NetworkHandler<ClientMsgGetHeaders>)
{
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

        let resp = match err {
            Some(err) => Response::Err(err.to_string()),
            None => Response::Ok(BlockHeaders(headers))
        };

        handler.sink.unbounded_send(
            Message::BlockHeaders(handler.identifier, resp)).unwrap();
    }

    handler.sink.unbounded_send(
        Message::CloseConnection(handler.identifier)).unwrap();
}

fn handle_get_blocks(
    blockchain: &BlockchainR,
    from: BlockHash,
    to: BlockHash,
    handler: NetworkHandler<ClientMsgGetBlocks>)
{
    let blockchain = blockchain.read().unwrap();

    for x in iter::Iter::new(&blockchain.get_storage(), from, to).unwrap() {
        let resp = match x {
            Err(err) => Response::Err(err.to_string()),
            Ok((rblk, blk)) => Response::Ok(blk) // FIXME: use rblk
        };
        handler.sink.unbounded_send(
            Message::Block(handler.identifier, resp)).unwrap();
    }

    handler.sink.unbounded_send(
        Message::CloseConnection(handler.identifier)).unwrap();
}
