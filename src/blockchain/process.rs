use crate::blockcfg::Header;
use crate::blockchain::chain;
use crate::intercom::BlockMsg;
use crate::rest::v0::node::stats::StatsCounter;
use crate::utils::task::TaskBroadcastBox;
use chain_core::property::{HasHeader, Header as _};

pub fn process(
    blockchain: &chain::BlockchainR,
    bquery: BlockMsg,
    network_broadcast: &mut TaskBroadcastBox<Header>,
    stats_counter: &StatsCounter,
) {
    let res = match bquery {
        BlockMsg::LeadershipBlock(block) => {
            let header = block.header();
            debug!("received block from the leadership: {:#?}", header);
            let res = blockchain.lock_write().handle_incoming_block(block);
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
                .lock_write()
                .handle_block_announcement(header.id());
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
