use super::super::intercom::{BlockMsg, NetworkBroadcastMsg};
use super::super::leadership::selection;
use crate::blockcfg::cardano::Cardano;
use futures::sync::mpsc::UnboundedSender;
use stats::SharedStats;
use std::sync::Arc;

use super::chain;

pub fn process(
    blockchain: &chain::BlockchainR<Cardano>,
    _selection: &Arc<selection::Selection>,
    bquery: BlockMsg<Cardano>,
    network_broadcast: &UnboundedSender<NetworkBroadcastMsg<Cardano>>,
    shared_stats: &SharedStats,
) {
    match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block);
            blockchain.write().unwrap().handle_incoming_block(block);
            shared_stats.incr_block_recv_cnt();
        }
        BlockMsg::LeadershipBlock(block) => {
            debug!("received block from the leadership: {:#?}", block);
            blockchain
                .write()
                .unwrap()
                .handle_incoming_block(block.clone());
            network_broadcast
                .unbounded_send(NetworkBroadcastMsg::Block(block))
                .unwrap();
        }
    }
}
