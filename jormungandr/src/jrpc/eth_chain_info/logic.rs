use super::Error;
use crate::{context::Context, jrpc::eth_types::sync::SyncStatus};
use chain_evm::ethereum_types::{U256, U64};

pub fn get_chain_id(_context: &Context) -> Result<Option<U64>, Error> {
    // TODO implement
    Ok(Some(0.into()))
}

pub fn is_syncing(_context: &Context) -> Result<SyncStatus, Error> {
    // TODO implement
    Ok(SyncStatus::build())
}

pub fn get_gas_price(_context: &Context) -> Result<U256, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn get_protocol_verion(_context: &Context) -> Result<u64, Error> {
    // TODO implement
    Ok(0)
}
