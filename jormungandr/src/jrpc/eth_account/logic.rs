use crate::{
    context::Context,
    jrpc::{
        eth_types::{block_number::BlockNumber, bytes::Bytes, number::Number},
        Error,
    },
};
use chain_evm::ethereum_types::{H160, H256};

pub fn accounts(context: &Context) -> Result<Vec<H160>, Error> {
    Ok(context
        .try_full()?
        .evm_keys
        .iter()
        .map(|secret_key| secret_key.address())
        .collect())
}

pub async fn get_transaction_count(
    address: H160,
    block_number: BlockNumber,
    context: &Context,
) -> Result<Number, Error> {
    match block_number {
        BlockNumber::Latest => {
            let ledger = context.blockchain_tip()?.get_ref().await.ledger();
            let address = ledger.get_jormungandr_mapped_address(&address);
            let account = ledger.accounts().get_state(&address)?;
            Ok(account.evm_state.nonce.into())
        }
        _ => Err(Error::NonArchiveNode),
    }
}

pub async fn get_balance(
    address: H160,
    block_number: BlockNumber,
    context: &Context,
) -> Result<Number, Error> {
    match block_number {
        BlockNumber::Latest => {
            let ledger = context.blockchain_tip()?.get_ref().await.ledger();
            let address = ledger.get_jormungandr_mapped_address(&address);
            let account = ledger.accounts().get_state(&address)?;
            Ok(account.value.0.into())
        }
        _ => Err(Error::NonArchiveNode),
    }
}

pub async fn get_code(
    address: H160,
    block_number: BlockNumber,
    context: &Context,
) -> Result<Bytes, Error> {
    match block_number {
        BlockNumber::Latest => {
            let ledger = context.blockchain_tip()?.get_ref().await.ledger();
            let address = ledger.get_jormungandr_mapped_address(&address);
            let account = ledger.accounts().get_state(&address)?;
            Ok(account.evm_state.code.clone().into())
        }
        _ => Err(Error::NonArchiveNode),
    }
}

pub async fn get_storage_at(
    address: H160,
    key: H256,
    block_number: BlockNumber,
    context: &Context,
) -> Result<H256, Error> {
    match block_number {
        BlockNumber::Latest => {
            let ledger = context.blockchain_tip()?.get_ref().await.ledger();
            let address = ledger.get_jormungandr_mapped_address(&address);
            let account = ledger.accounts().get_state(&address)?;
            Ok(account
                .evm_state
                .storage
                .get(&key)
                .cloned()
                .unwrap_or_default())
        }
        _ => Err(Error::NonArchiveNode),
    }
}
