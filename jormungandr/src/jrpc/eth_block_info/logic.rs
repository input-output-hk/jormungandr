use super::Error;
use crate::{
    context::Context,
    jrpc::eth_types::{block::Block, block_number::BlockNumber, number::Number},
};
use chain_evm::ethereum_types::H256;

pub async fn get_block_by_hash(
    hash: H256,
    full: bool,
    context: &Context,
) -> Result<Option<Block>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    blockchain_tip.ledger().evm_block_gas_limit();
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(block.map(|block| Block::build(block, full)))
}

pub fn get_block_by_number(
    _number: BlockNumber,
    full: bool,
    context: &Context,
) -> Result<Option<Block>, Error> {
    // TODO implement
    let block = context.blockchain()?.storage().get([0; 32].into())?;
    Ok(block.map(|block| Block::build(block, full)))
}

pub fn get_transaction_count_by_hash(
    _hash: H256,
    _context: &Context,
) -> Result<Option<Number>, Error> {
    // TODO implement
    Ok(Some(0.into()))
}

pub fn get_transaction_count_by_number(
    _number: BlockNumber,
    _context: &Context,
) -> Result<Option<Number>, Error> {
    // TODO implement
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_hash(_: H256, _: &Context) -> Result<Option<Number>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_number(_: BlockNumber, _: &Context) -> Result<Option<Number>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub fn get_block_number(_: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}
