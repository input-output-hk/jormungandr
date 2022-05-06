use chain_evm::ethereum_types::{H256, H64};

use crate::{
    context::Context,
    jrpc::eth_types::{number::Number, work::Work},
};

use super::Error;

pub fn mining(_context: &Context) -> Result<bool, Error> {
    // TODO implement
    Ok(true)
}

pub fn hashrate(_context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn get_work(_context: &Context) -> Result<Work, Error> {
    // TODO implement
    Ok(Work::build())
}

pub fn submit_work(
    _nonce: H64,
    _pow_hash: H256,
    _mix_digest: H256,
    _context: &Context,
) -> Result<bool, Error> {
    // TODO implement
    Ok(true)
}

pub fn submit_hashrate(_hash_rate: H256, _id: H256, _context: &Context) -> Result<bool, Error> {
    // TODO implement
    Ok(true)
}
