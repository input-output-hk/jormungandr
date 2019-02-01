use super::super::intercom::{BlockMsg, NetworkBroadcastMsg};
use super::super::leadership::selection;
use crate::blockcfg::{mock::Mockchain, BlockConfig};
use futures::sync::mpsc::UnboundedSender;
use std::sync::Arc;

use super::chain;

pub fn process<Chain>(
    blockchain: &chain::BlockchainR<Chain>,
    _selection: &Arc<selection::Selection>,
    bquery: BlockMsg<Chain>,
    network_broadcast: &UnboundedSender<NetworkBroadcastMsg<Chain>>,
) where
    Chain: BlockConfig,
    <Chain as BlockConfig>::Block: std::fmt::Debug + Clone,
{
    match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block);
            blockchain.write().unwrap().handle_incoming_block(block);
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
