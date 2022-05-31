use super::Error;
use crate::{
    context::Context,
    intercom::{self, TransactionMsg},
    jrpc::eth_types::{
        block_number::BlockNumber, bytes::Bytes, number::Number, receipt::Receipt,
        transaction::Transaction,
    },
};
use chain_evm::ethereum_types::{H160, H256, H512};
use chain_impl_mockchain::fragment::Fragment;
use jormungandr_lib::interfaces::FragmentOrigin;

pub async fn send_transaction(tx: Transaction, context: &Context) -> Result<H256, Error> {
    let fragment = Fragment::Evm(tx.into());
    let (reply_handle, reply_future) = intercom::unary_reply();
    let msg = TransactionMsg::SendTransactions {
        origin: FragmentOrigin::JRpc,
        fragments: vec![fragment],
        fail_fast: true,
        reply_handle,
    };

    context.try_full()?.transaction_task.clone().try_send(msg)?;
    let reply = reply_future.await?;
    if reply.is_error() {
        Err(Error::Fragment(reply))
    } else {
        Ok(H256::zero())
    }
}

pub async fn send_raw_transaction(_raw_tx: Bytes, context: &Context) -> Result<H256, Error> {
    let fragment = Fragment::Initial(Default::default());
    let (reply_handle, reply_future) = intercom::unary_reply();
    let msg = TransactionMsg::SendTransactions {
        origin: FragmentOrigin::JRpc,
        fragments: vec![fragment],
        fail_fast: true,
        reply_handle,
    };

    context.try_full()?.transaction_task.clone().try_send(msg)?;
    let reply = reply_future.await?;
    if reply.is_error() {
        Err(Error::Fragment(reply))
    } else {
        Ok(H256::zero())
    }
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
