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
        BlockMsg::LeadershipBlock(block) => {
            let header = block.header();
            debug!("received block from the leadership: {:#?}", header);
            let res = blockchain.write().unwrap().handle_incoming_block(block);
            network_broadcast.send_broadcast(header);
            res
        }
        BlockMsg::Subscribe(reply) => {
            let rx = network_broadcast.add_rx();
            reply.send(rx);
            Ok(())
        }
        BlockMsg::AnnouncedBlock(header) => {
            debug!("received block header from the network: {:#?}", header);
            let res = blockchain
                .write()
                .unwrap()
                .handle_block_announcement(header);
            if res.is_ok() {
                stats_counter.add_block_recv_cnt(1);
            }
            res
        }
    };
    if let Err(e) = res {
        error!("error processing an incoming block: {:?}", e);
    }
}
