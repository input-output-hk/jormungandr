use super::super::intercom::BlockMsg;

use super::chain;

pub fn process(blockchain: &chain::BlockchainR, bquery: BlockMsg) {
    debug!("block msg received: {:#?}", bquery);
}
