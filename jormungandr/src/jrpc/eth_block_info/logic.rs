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
    let gas_limit = blockchain_tip.ledger().evm_block_gas_limit();
    let gas_price = blockchain_tip.ledger().evm_gas_price();
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(block.map(|block| Block::build(block, full, gas_limit, gas_price)))
}

pub async fn get_block_by_number(
    number: BlockNumber,
    full: bool,
    context: &Context,
) -> Result<Option<Block>, Error> {
    let blockchain = context.blockchain()?;
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_limit = blockchain_tip.ledger().evm_block_gas_limit();
    let gas_price = blockchain_tip.ledger().evm_gas_price();
    match number {
        BlockNumber::Latest => {
            let block = blockchain.storage().get(blockchain_tip.hash())?;
            Ok(block.map(|block| Block::build(block, full, gas_limit, gas_price)))
        }
        BlockNumber::Earliest => {
            let block = blockchain.storage().get(*blockchain.block0())?;
            Ok(block.map(|block| Block::build(block, full, gas_limit, gas_price)))
        }
        BlockNumber::Pending => Ok(None),
        BlockNumber::Num(number) if number <= blockchain_tip.chain_length().into() => {
            let distance = Into::<u32>::into(blockchain_tip.chain_length()) - number;
            let block = blockchain
                .storage()
                .get_nth_ancestor(blockchain_tip.hash(), distance)?;
            Ok(block.map(|block| Block::build(block, full, gas_limit, gas_price)))
        }
        BlockNumber::Num(_) => Ok(None),
    }
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
