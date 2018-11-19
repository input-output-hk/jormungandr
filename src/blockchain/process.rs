use futures::sync::mpsc::UnboundedSender;
use super::super::intercom::{BlockMsg, NetworkBroadcastMsg};
use super::super::leadership::selection;
use std::sync::Arc;

use super::chain;

pub fn process(
    blockchain: &chain::BlockchainR,
    selection: &Arc<selection::Selection>,
    bquery: BlockMsg,
    network_broadcast: &UnboundedSender<NetworkBroadcastMsg>
)
{
    match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block);
            blockchain.write().unwrap().handle_incoming_block(block);
        }
        BlockMsg::LeadershipBlock(block) => {
            debug!("received block from the leadership: {:#?}", block);
            blockchain.write().unwrap().handle_incoming_block(block.clone());
            network_broadcast.unbounded_send(NetworkBroadcastMsg::Block(block)).unwrap();
        }
    }
}
