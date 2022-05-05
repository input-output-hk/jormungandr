use super::Error;
use crate::context::Context;
use chain_evm::ethereum_types::U256;

pub fn new_filter(_context: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn new_block_filter(_context: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn new_pending_transaction_filter(_context: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn uninstall_filter(_context: &Context) -> Result<bool, Error> {
    // TODO implement
    Ok(true)
}

pub fn get_filter_changes(_context: &Context) -> Result<(), Error> {
    Ok(())
}

pub fn get_filter_logs(_context: &Context) -> Result<(), Error> {
    Ok(())
}
