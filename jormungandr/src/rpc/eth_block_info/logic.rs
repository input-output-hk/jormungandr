use super::Error;
use crate::{context::Context, rpc::eth_types::block::Block};
use chain_evm::ethereum_types::H256;

pub fn get_block_by_hash(hash: H256, full: bool, context: &Context) -> Result<Option<Block>, Error> {
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(block.map(|block| Block::build(block, full)))
}
