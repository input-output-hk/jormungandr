use crate::{
    context::Context,
    intercom::{self, TransactionMsg},
    jrpc::{
        eth_block_info::get_block_by_number_from_context,
        eth_types::{
            block::Block, block_number::BlockNumber, bytes::Bytes, number::Number,
            receipt::Receipt, transaction::Transaction,
        },
        Error,
    },
};
use chain_evm::{
    ethereum_types::{H160, H256, H512},
    transaction::{EthereumSignedTransaction, EthereumUnsignedTransaction},
};
use chain_impl_mockchain::{block::Block as JorBlock, fragment::Fragment};
use jormungandr_lib::interfaces::FragmentOrigin;

fn get_transaction_from_block_by_index(
    block: Option<JorBlock>,
    index: Number,
    gas_price: u64,
) -> Option<Transaction> {
    match &block {
        Some(block) => Block::get_transaction_by_index(block, u64::from(index.clone()) as usize)
            .map(|tx| {
                Transaction::build(
                    tx,
                    Some(H256::from_slice(block.header().hash().as_bytes())),
                    Some((u32::from(block.header().chain_length()) as u64).into()),
                    Some(index),
                    gas_price,
                )
            }),
        None => None,
    }
}

pub async fn send_transaction(tx: Transaction, context: &Context) -> Result<H256, Error> {
    let fragment = Fragment::Evm(tx.into());
    let (reply_handle, reply_future) = intercom::unary_reply();
    let msg = TransactionMsg::SendTransactions {
        origin: FragmentOrigin::JRpc,
        fragments: vec![fragment],
        fail_fast: true,
        reply_handle,
    };

    context
        .try_full()?
        .transaction_task
        .clone()
        .try_send(msg)
        .map_err(Box::new)?;
    let reply = reply_future.await?;
    if reply.is_error() {
        Err(Error::Fragment(reply))
    } else {
        Ok(H256::zero())
    }
}

pub async fn send_raw_transaction(raw_tx: Bytes, context: &Context) -> Result<H256, Error> {
    let tx = EthereumSignedTransaction::from_bytes(raw_tx.as_ref())
        .map_err(|e| Error::TransactionDecodedErorr(e.to_string()))?;
    let fragment = Fragment::Evm(tx.try_into().map_err(Error::TransactionDecodedErorr)?);
    let (reply_handle, reply_future) = intercom::unary_reply();
    let msg = TransactionMsg::SendTransactions {
        origin: FragmentOrigin::JRpc,
        fragments: vec![fragment],
        fail_fast: true,
        reply_handle,
    };

    context
        .try_full()?
        .transaction_task
        .clone()
        .try_send(msg)
        .map_err(Box::new)?;
    let reply = reply_future.await?;
    if reply.is_error() {
        Err(Error::Fragment(reply))
    } else {
        Ok(H256::zero())
    }
}

pub async fn get_transaction_by_hash(
    _hash: H256,
    _context: &Context,
) -> Result<Option<Transaction>, Error> {
    // TODO implement
    Err(Error::NonArchiveNode)
}

pub async fn get_transaction_by_block_hash_and_index(
    hash: H256,
    index: Number,
    context: &Context,
) -> Result<Option<Transaction>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().get_evm_gas_price();
    let block = context.blockchain()?.storage().get(hash.0.into())?;
    Ok(get_transaction_from_block_by_index(block, index, gas_price))
}

pub async fn get_transaction_by_block_number_and_index(
    number: BlockNumber,
    index: Number,
    context: &Context,
) -> Result<Option<Transaction>, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    let gas_price = blockchain_tip.ledger().get_evm_gas_price();
    let blockchain = context.blockchain()?;
    let block = get_block_by_number_from_context(number, blockchain, blockchain_tip).unwrap();
    Ok(get_transaction_from_block_by_index(block, index, gas_price))
}

pub fn get_transaction_receipt(_hash: H256, _context: &Context) -> Result<Option<Receipt>, Error> {
    // TODO implement
    Err(Error::NonArchiveNode)
}

pub fn sign_transaction(raw_tx: Bytes, _context: &Context) -> Result<Bytes, Error> {
    // FIXME: the account secret is assumed to be obtained by an internal keystore
    //        THIS IMPLEMENTATION GENERATES A RANDOM SECRET EACH TIME, and is meant
    //        to be a placeholder that mocks getting the account secret. DO NOT RELY
    //        ON THIS METHOD UNTIL A SECURE KEYSTORE MECHANISM IS IN PLACE.
    let account_secret = chain_impl_mockchain::evm::util::generate_account_secret();
    let tx = EthereumUnsignedTransaction::from_bytes(raw_tx.as_ref())
        .map_err(|e| Error::TransactionDecodedErorr(e.to_string()))?;
    let signed = account_secret.sign(tx)?;
    Ok(Bytes::from(signed.to_bytes().into_boxed_slice()))
}

pub async fn estimate_gas(tx: Transaction, context: &Context) -> Result<Number, Error> {
    let blockchain_tip = context.blockchain_tip()?.get_ref().await;
    Ok(blockchain_tip
        .ledger()
        .estimate_evm_transaction(tx.into())
        .map_err(Box::new)?
        .into())
}

pub fn sign(_address: H160, _message: Bytes, _context: &Context) -> Result<H512, Error> {
    // TODO implement
    Ok(H512::zero())
}

pub fn call(_tx: Transaction, _number: BlockNumber, _context: &Context) -> Result<Bytes, Error> {
    // TODO implement
    Ok(Default::default())
}
