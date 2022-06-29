use crate::{
    context::Context,
    jrpc::{
        eth_types::{block_number::BlockNumber, fee::FeeHistory, number::Number, sync::SyncStatus},
        Error,
    },
};

pub fn chain_id(_: &Context) -> Result<Number, Error> {
    // In Ethereum it was added on EIP-155 https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md as a transaction replay protection
    // Another description of this field https://ethereum.stackexchange.com/questions/37533/what-is-a-chainid-in-ethereum-how-is-it-different-than-networkid-and-how-is-it
    //
    // As we are not compatible with the original Ethereum network protocol, we can return any value at this case
    Ok(1.into())
}

pub async fn syncing(context: &Context) -> Result<SyncStatus, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let chain_length: u32 = blockchain_tip.chain_length().into();

    // We are considering that jormungandr node is always syncing from the genesisng block (height: 0) till the current known block
    Ok(SyncStatus::build(
        0,
        chain_length.into(),
        chain_length.into(),
    ))
}

pub async fn gas_price(context: &Context) -> Result<Number, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().get_evm_gas_price();
    Ok(gas_price.into())
}

pub fn protocol_verion(_: &Context) -> Result<u64, Error> {
    Ok(1)
}

pub fn fee_history(
    _block_count: Number,
    _newest_block: BlockNumber,
    _reward_percentiles: Vec<f64>,
    _context: &Context,
) -> Result<FeeHistory, Error> {
    Err(Error::NonArchiveNode)
}
