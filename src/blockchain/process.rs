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
        }
        BlockMsg::LeadershipBlock(block) => {
            network_broadcast.unbounded_send(NetworkBroadcastMsg::Block(block.clone())).unwrap();
           debug!("received block from the leadership: {:#?}", block);
        }
    }
}
