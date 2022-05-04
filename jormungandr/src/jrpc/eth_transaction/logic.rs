use super::Error;
use crate::{
    context::Context,
    jrpc::eth_types::{
        block_number::BlockNumber, bytes::Bytes, index::Index, receipt::Receipt,
        transaction::Transaction,
    },
};
use chain_evm::ethereum_types::H256;

pub fn send_transaction(_tx: Transaction, _context: &Context) -> Result<H256, Error> {
    // TODO implement
    Ok(H256::zero())
}

pub fn send_raw_transaction(_raw_tx: Bytes, _context: &Context) -> Result<H256, Error> {
    // TODO implement
    Ok(H256::zero())
}

pub fn get_transaction_by_hash(
    _hash: H256,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Ok(Some(Transaction::build()))
}

pub fn get_transaction_by_block_hash_and_index(
    _hash: H256,
    _index: Index,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Ok(Some(Transaction::build()))
}

pub fn get_transaction_by_block_number_and_index(
    _number: BlockNumber,
    _index: Index,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Ok(Some(Transaction::build()))
}

pub fn get_transaction_receipt(_hash: H256, _context: &Context) -> Result<Option<Receipt>, Error> {
    // TODO implement
    Ok(Some(Receipt::build()))
}
