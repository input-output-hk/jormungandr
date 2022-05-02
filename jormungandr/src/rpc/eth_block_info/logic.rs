use super::Error;
use crate::{
    context::Context,
    rpc::eth_types::{block::Block, block_number::BlockNumber},
};
use chain_evm::ethereum_types::{H256, U256};

pub fn get_block_by_hash(
    _hash: H256,
    full: bool,
    _context: &Context,
) -> Result<Option<Block>, Error> {
    // TODO implement
    Ok(Some(Block::build(full)))
}

pub fn get_block_by_number(
    _number: BlockNumber,
    _full: bool,
    _context: &Context,
) -> Result<Option<Block>, Error> {
    // TODO implement
    Ok(Some(Block::default()))
}

pub fn get_transaction_count_by_hash(
    _hash: H256,
    _context: &Context,
) -> Result<Option<U256>, Error> {
    // TODO implement
    Ok(Some(0.into()))
}

pub fn get_transaction_count_by_number(
    _number: BlockNumber,
    _context: &Context,
) -> Result<Option<U256>, Error> {
    // TODO implement
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_hash(_: H256, _: &Context) -> Result<Option<U256>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_number(_: BlockNumber, _: &Context) -> Result<Option<U256>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub fn get_block_number(_: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}
