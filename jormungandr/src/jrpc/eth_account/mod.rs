use jsonrpsee_http_server::RpcModule;

use crate::context::ContextLock;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_account_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_getTransactionCount", |_params, context| async move {
            let context = context.read().await;
            // let (block_hash, full) = params.parse()?;
            logic::get_transaction_count(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getBalance", |_params, context| async move {
            let context = context.read().await;
            // let (block_hash, full) = params.parse()?;
            logic::get_balance(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getCode", |_params, context| async move {
            let context = context.read().await;
            // let (block_hash, full) = params.parse()?;
            logic::get_code(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getStorageAt", |_params, context| async move {
            let context = context.read().await;
            // let (block_hash, full) = params.parse()?;
            logic::get_storage_at(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
