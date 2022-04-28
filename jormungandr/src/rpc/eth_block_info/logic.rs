use super::Error;
use crate::{context::Context, rpc::eth_types::block::Block};
use chain_evm::ethereum_types::{H256, U256};

pub fn get_block_by_hash(
    hash: H256,
    full: bool,
    context: &Context,
) -> Result<Option<Block>, Error> {
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(block.map(|block| Block::build(block, full)))
}

pub fn get_block_by_number(
    _number: u64,
    _full: bool,
    _context: &Context,
) -> Result<Option<Block>, Error> {
    // TODO implement
    Ok(None)
}

pub fn get_transaction_count_by_hash(hash: H256, context: &Context) -> Result<Option<U256>, Error> {
    let block = context.blockchain()?.storage().get(hash.0.into())?;

    let count = block.map_or(0, |block| {
        let mut count = 0;
        block.fragments().for_each(|_| count += 1);
        count
    });

    Ok(Some(count.into()))
}

pub fn get_transaction_count_by_number(
    _number: u64,
    _context: &Context,
) -> Result<Option<U256>, Error> {
    // TODO implement
    Ok(None)
}

pub fn get_uncle_count_by_hash(_: H256, _: &Context) -> Result<Option<U256>, Error> {
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_number(_: u64, _: &Context) -> Result<Option<U256>, Error> {
    Ok(Some(0.into()))
}

pub fn get_block_number(_: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}
