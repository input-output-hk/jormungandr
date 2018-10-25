use super::super::intercom::BlockMsg;

use super::chain;

pub fn process(blockchain: &chain::BlockchainR, bquery: BlockMsg) {
    println!("block msg received: {}", bquery);
}
