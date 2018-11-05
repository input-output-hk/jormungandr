use blockchain::{BlockchainR};
use cardano_storage::{block_read};
use intercom::*;
use protocol::{Message, protocol::{Response, BlockHeaders}};
use std::sync::{mpsc::Receiver};

pub fn client_task(blockchain: BlockchainR, r: Receiver<ClientMsg>) {
    loop {
        let query = r.recv().unwrap();
        debug!("client query received: {:?}", query);

        match query {
            ClientMsg::GetBlockTip(handler) => handle_get_block_tip(&blockchain, handler),
            _ => unimplemented!()
        }
    }
}

fn handle_get_block_tip(blockchain: &BlockchainR, handler: NetworkHandler<ClientMsgGetHeaders>) {
    let blockchain = blockchain.read().unwrap();
    let tip = blockchain.get_tip();
    let resp = match block_read(blockchain.get_storage(), &tip) {
        None => Response::Err(format!("Cannot read block '{}'", tip)),
        Some(rblk) => {
            let blk = rblk.decode().unwrap();
            Response::Ok(BlockHeaders(vec![blk.get_header()]))
        }
    };
    handler.sink.unbounded_send(
        Message::BlockHeaders(handler.identifier, resp)).unwrap();
}
