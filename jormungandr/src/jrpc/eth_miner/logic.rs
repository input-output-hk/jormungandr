use crate::{
    context::Context,
    jrpc::{
        eth_types::{number::Number, work::Work},
        Error,
    },
};
use chain_evm::ethereum_types::{H160, H256, H64};

pub fn mining(_context: &Context) -> Result<bool, Error> {
    Err(Error::MiningIsNotAllowed)
}

pub fn coinbase(_context: &Context) -> Result<H160, Error> {
    Err(Error::MiningIsNotAllowed)
}

pub fn hashrate(_context: &Context) -> Result<Number, Error> {
    Err(Error::MiningIsNotAllowed)
}

pub fn get_work(_context: &Context) -> Result<Work, Error> {
    Err(Error::MiningIsNotAllowed)
}

pub fn submit_work(
    _nonce: H64,
    _pow_hash: H256,
    _mix_digest: H256,
    _context: &Context,
) -> Result<bool, Error> {
    Err(Error::MiningIsNotAllowed)
}

pub fn submit_hashrate(_hash_rate: H256, _id: H256, _context: &Context) -> Result<bool, Error> {
    Err(Error::MiningIsNotAllowed)
}
