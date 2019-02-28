use crate::blockcfg::BlockConfig;
use crate::blockchain::chain;
use crate::intercom::BlockMsg;
use crate::rest::v0::node::stats::StatsCounter;
use crate::utils::task::TaskBroadcastBox;

use chain_core::property::{self, HasHeader};

use std::fmt::Debug;

pub fn process<Chain>(
    blockchain: &chain::BlockchainR<Chain>,
    bquery: BlockMsg<Chain>,
    network_broadcast: &mut TaskBroadcastBox<Chain::BlockHeader>,
    stats_counter: &StatsCounter,
) where
    Chain: BlockConfig,
    Chain::BlockHeader: Debug,
    <Chain::Ledger as property::Ledger>::Update: Clone,
    <Chain::Settings as property::Settings>::Update: Clone,
    <Chain::Leader as property::LeaderSelection>::Update: Clone,
{
    let res = match bquery {
        BlockMsg::NetworkBlock(block) => {
            debug!("received block from the network: {:#?}", block.header());
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            if res.is_ok() {
                stats_counter.add_block_recv_cnt(1);
            }
            res
        }
        BlockMsg::LeadershipBlock(block) => {
            let header = block.header();
            debug!("received block from the leadership: {:#?}", header);
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            network_broadcast.send_broadcast(header);
            res
        }
        BlockMsg::Subscribe(_reply) => unimplemented!(),
    };
    if let Err(e) = res {
        error!("error processing an incoming block: {:?}", e);
    }
}
