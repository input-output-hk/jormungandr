use crate::{
    context::Context,
    jrpc::{
        eth_block_info::get_block_by_number_from_context,
        eth_types::{
            block::Block, block_number::BlockNumber, bytes::Bytes, number::Number,
            receipt::Receipt, transaction::Transaction,
        },
        Error,
    },
};
use chain_evm::ethereum_types::{H160, H256, H512};
use chain_impl_mockchain::block::Block as JorBlock;

fn get_transaction_from_block_by_index(
    block: Option<JorBlock>,
    index: Number,
    gas_price: u64,
) -> Option<Transaction> {
    match &block {
        Some(block) => Block::get_transaction_by_index(block, u64::from(index.clone()) as usize)
            .map(|tx| {
                Transaction::build(
                    tx,
                    Some(H256::from_slice(block.header().hash().as_bytes())),
                    Some((u32::from(block.header().chain_length()) as u64).into()),
                    Some(index),
                    gas_price,
                )
            }),
        None => None,
    }
}

pub fn send_transaction(_tx: Transaction, _context: &Context) -> Result<H256, Error> {
    // TODO implement
    Ok(H256::zero())
}

pub fn send_raw_transaction(_raw_tx: Bytes, _context: &Context) -> Result<H256, Error> {
    // TODO implement
    Ok(H256::zero())
}

pub async fn get_transaction_by_hash(
    _hash: H256,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Err(Error::NonArchiveNode)
}

pub async fn get_transaction_by_block_hash_and_index(
    hash: H256,
    index: Number,
    context: &Context,
) -> Result<Option<Transaction>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().evm_gas_price();
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(get_transaction_from_block_by_index(block, index, gas_price))
}

pub async fn get_transaction_by_block_number_and_index(
    number: BlockNumber,
    index: Number,
    context: &Context,
) -> Result<Option<Transaction>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().evm_gas_price();
    let blockchain = context.blockchain()?;
    let block = get_block_by_number_from_context(number, blockchain, blockchain_tip).unwrap();
    Ok(get_transaction_from_block_by_index(block, index, gas_price))
}

pub fn get_transaction_receipt(_hash: H256, _context: &Context) -> Result<Option<Receipt>, Error> {
    // TODO implement
    Err(Error::NonArchiveNode)
}

pub fn sign_transaction(_tx: Transaction, _context: &Context) -> Result<Bytes, Error> {
    // TODO implement
    Ok(Default::default())
}

pub fn estimate_gas(_tx: Transaction, _context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn sign(_address: H160, _message: Bytes, _context: &Context) -> Result<H512, Error> {
    // TODO implement
    Ok(H512::zero())
}

pub fn call(_tx: Transaction, _number: BlockNumber, _context: &Context) -> Result<Bytes, Error> {
    // TODO implement
    Ok(Default::default())
}
