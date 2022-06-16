use crate::{
    context::Context,
    jrpc::{
        eth_types::{block_number::BlockNumber, fee::FeeHistory, number::Number, sync::SyncStatus},
        Error,
    },
};

pub fn chain_id(_context: &Context) -> Result<Number, Error> {
    // TODO implement
    Ok(0.into())
}

pub fn syncing(_context: &Context) -> Result<SyncStatus, Error> {
    // TODO implement
    Ok(SyncStatus::build())
}

pub async fn gas_price(context: &Context) -> Result<Number, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().evm_gas_price();
    Ok(gas_price.into())
}

pub fn protocol_verion(_context: &Context) -> Result<u64, Error> {
    // TODO implement
    Ok(0)
}

pub fn fee_history(
    _block_count: Number,
    _newest_block: BlockNumber,
    _reward_percentiles: Vec<f64>,
    _context: &Context,
) -> Result<FeeHistory, Error> {
    // TODO implement
    Ok(FeeHistory::build())
}
