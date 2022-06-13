use crate::{
    blockchain::{Blockchain, Ref},
    context::Context,
    jrpc::{
        eth_types::{block::Block, block_number::BlockNumber, number::Number},
        Error,
    },
};
use chain_evm::ethereum_types::H256;
use chain_impl_mockchain::block::Block as JorBlock;
use std::sync::Arc;

pub fn get_block_by_number_from_context(
    number: BlockNumber,
    blockchain: &Blockchain,
    blockchain_tip: Arc<Ref>,
) -> Result<Option<JorBlock>, Error> {
    match number {
        BlockNumber::Latest => {
            let block = blockchain.storage().get(blockchain_tip.hash())?;
            Ok(block)
        }
        BlockNumber::Earliest => {
            let block = blockchain.storage().get(*blockchain.block0())?;
            Ok(block)
        }
        BlockNumber::Pending => Ok(None),
        BlockNumber::Num(number) if number <= blockchain_tip.chain_length().into() => {
            let distance = u32::from(blockchain_tip.chain_length()) - number;
            let block = blockchain
                .storage()
                .get_nth_ancestor(blockchain_tip.hash(), distance)?;
            Ok(block)
        }
        BlockNumber::Num(_) => Ok(None),
    }
}

pub async fn get_block_by_hash(
    hash: H256,
    full: bool,
    context: &Context,
) -> Result<Option<Block>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_limit = blockchain_tip.ledger().get_evm_block_gas_limit();
    let gas_price = blockchain_tip.ledger().get_evm_gas_price();
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
    let gas_limit = blockchain_tip.ledger().get_evm_block_gas_limit();
    let gas_price = blockchain_tip.ledger().get_evm_gas_price();
    Ok(
        get_block_by_number_from_context(number, blockchain, blockchain_tip)?
            .map(|block| Block::build(block, full, gas_limit, gas_price)),
    )
}

pub fn get_transaction_count_by_hash(
    hash: H256,
    context: &Context,
) -> Result<Option<Number>, Error> {
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(block.map(Block::calc_transactions_count))
}

pub async fn get_transaction_count_by_number(
    number: BlockNumber,
    context: &Context,
) -> Result<Option<Number>, Error> {
    let blockchain = context.blockchain()?;
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    Ok(
        get_block_by_number_from_context(number, blockchain, blockchain_tip)?
            .map(Block::calc_transactions_count),
    )
}

pub fn get_uncle_count_by_hash(_: H256, _: &Context) -> Result<Option<Number>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub fn get_uncle_count_by_number(_: BlockNumber, _: &Context) -> Result<Option<Number>, Error> {
    // jormungandr block does not have any ethereum "uncles" so we allways return 0
    Ok(Some(0.into()))
}

pub async fn get_block_number(context: &Context) -> Result<Number, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    Ok((Into::<u32>::into(blockchain_tip.chain_length()) as u64).into())
}
