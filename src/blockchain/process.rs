use chain_core::property;
use crate::blockcfg::BlockConfig;
use crate::blockchain::chain;
use crate::intercom::{BlockMsg, NetworkBroadcastMsg};
use futures::sync::mpsc::UnboundedSender;
use stats::SharedStats;
use std::sync::Arc;

pub fn process<Chain>(
    blockchain: &chain::BlockchainR<Chain>,
    bquery: BlockMsg<Chain>,
    network_broadcast: &UnboundedSender<NetworkBroadcastMsg<Chain>>,
    shared_stats: &SharedStats,
) where
    Chain: BlockConfig,
    <Chain as BlockConfig>::Block: std::fmt::Debug + Clone,
    <Chain::Ledger as property::Ledger>::Update: Clone,
    <Chain::Settings as property::Settings>::Update: Clone,
    <Chain::Leader as property::LeaderSelection>::Update: Clone,
    for<'a> &'a <Chain::Block as property::HasTransaction>::Transactions:
        IntoIterator<Item = &'a Chain::Transaction>,
{
    let res = match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block);
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            if res.is_ok() {
                shared_stats.add_block_recv_cnt(1);
            }
            res
        }
        BlockMsg::LeadershipBlock(block) => {
            debug!("received block from the leadership: {:#?}", block);
            let res = blockchain
                .write()
                .unwrap()
                .handle_incoming_block(block.clone());
            network_broadcast
                .unbounded_send(NetworkBroadcastMsg::Block(block))
                .unwrap();
            res
        }
    };
    if let Err(e) = res {
        error!("error processing an incoming block: {:?}", e);
    }
}
