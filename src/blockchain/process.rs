use crate::blockcfg::BlockConfig;
use crate::blockchain::chain;
use crate::intercom::{BlockMsg, NetworkBroadcastMsg};
use chain_core::property;
use futures::sync::mpsc::UnboundedSender;
use rest::v0_node_stats::StatsCounter;

pub fn process<Chain>(
    blockchain: &chain::BlockchainR<Chain>,
    bquery: BlockMsg<Chain>,
    network_broadcast: &UnboundedSender<NetworkBroadcastMsg<Chain>>,
    stats_counter: &StatsCounter,
) where
    Chain: BlockConfig,
    <Chain as BlockConfig>::Block: std::fmt::Debug + Clone,
    <Chain::Ledger as property::Ledger>::Update: Clone,
    <Chain::Settings as property::Settings>::Update: Clone,
    <Chain::Leader as property::LeaderSelection>::Update: Clone,
{
    let res = match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block);
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            if res.is_ok() {
                stats_counter.add_block_recv_cnt(1);
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
        BlockMsg::Subscribe(_reply) => unimplemented!(),
    };
    if let Err(e) = res {
        error!("error processing an incoming block: {:?}", e);
    }
}
