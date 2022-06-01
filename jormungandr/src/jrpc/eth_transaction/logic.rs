use crate::{
    context::Context,
    jrpc::{
        eth_types::{
            block_number::BlockNumber, bytes::Bytes, number::Number, receipt::Receipt,
            transaction::Transaction,
        },
        Error,
    },
};
use chain_evm::ethereum_types::{H160, H256, H512};

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
    Ok(None)
}

pub fn get_transaction_by_block_hash_and_index(
    _hash: H256,
    _index: Number,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Ok(None)
}

pub fn get_transaction_by_block_number_and_index(
    _number: BlockNumber,
    _index: Number,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Ok(None)
}

pub fn get_transaction_receipt(_hash: H256, _context: &Context) -> Result<Option<Receipt>, Error> {
    // TODO implement
    Ok(Some(Receipt::build()))
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
