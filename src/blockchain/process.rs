use super::super::intercom::BlockMsg;

use super::chain;

pub fn process(blockchain: &chain::BlockchainR, bquery: BlockMsg) {
    match bquery {
        BlockMsg::NetworkBlock(block) => {
           debug!("received block from the network: {:#?}", block);
        }
        BlockMsg::LeadershipBlock(block) => {
           debug!("received block from the leadership: {:#?}", block);
        }
    }
}
