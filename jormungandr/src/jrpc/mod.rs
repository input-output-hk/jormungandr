#[cfg(feature = "evm")]
mod eth_account;
#[cfg(feature = "evm")]
mod eth_block_info;
#[cfg(feature = "evm")]
mod eth_chain_info;
#[cfg(feature = "evm")]
mod eth_filter;
#[cfg(feature = "evm")]
mod eth_miner;
#[cfg(feature = "evm")]
mod eth_transaction;
#[cfg(feature = "evm")]
mod eth_types;

use crate::{
    context::ContextLock,
    intercom::{self, TransactionMsg},
};
use futures::channel::mpsc::TrySendError;
use jormungandr_lib::interfaces::FragmentsProcessingSummary;
use jsonrpsee_http_server::{HttpServerBuilder, RpcModule};
use std::net::SocketAddr;
use thiserror::Error;

pub struct Config {
    pub listen: SocketAddr,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ContextError(#[from] crate::context::Error),
    #[error(transparent)]
    Storage(#[from] crate::blockchain::StorageError),
    #[error("Currently we dont support archive and full modes, so unfortunately this functionality is not working at this moment")]
    NonArchiveNode,
    #[error(transparent)]
    IntercomError(#[from] intercom::Error),
    #[error(transparent)]
    TxMsgSendError(#[from] Box<TrySendError<TransactionMsg>>),
    #[error("Could not process fragment")]
    Fragment(FragmentsProcessingSummary),
    #[error("Cound not decode Ethereum transaction bytes, erorr: {0}")]
    TransactionDecodedErorr(String),
    #[error("We are not supporting mining functionality")]
    MinningAreNotAllowed,
}

pub async fn start_jrpc_server(config: Config, _context: ContextLock) {
    let server = HttpServerBuilder::default()
        .build(config.listen)
        .await
        .unwrap();

    #[allow(unused_mut)]
    let mut modules = RpcModule::new(());

    #[cfg(feature = "evm")]
    {
        modules
            .merge(eth_block_info::eth_block_info_module(_context.clone()))
            .unwrap();

        modules
            .merge(eth_chain_info::eth_chain_info_module(_context.clone()))
            .unwrap();

        modules
            .merge(eth_transaction::eth_transaction_module(_context.clone()))
            .unwrap();

        modules
            .merge(eth_account::eth_account_module(_context.clone()))
            .unwrap();

        modules
            .merge(eth_filter::eth_filter_module(_context.clone()))
            .unwrap();

        modules
            .merge(eth_miner::eth_miner_module(_context))
            .unwrap();
    }

    server.start(modules).unwrap().await
}
