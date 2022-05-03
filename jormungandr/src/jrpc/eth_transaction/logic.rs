use super::Error;
use crate::{context::Context, jrpc::eth_types::transaction::Transaction};
use chain_evm::ethereum_types::H256;

pub fn send_transaction(_tx: Transaction, _context: &Context) -> Result<H256, Error> {
    // TODO implement
    Ok(H256::zero())
}
