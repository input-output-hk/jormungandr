use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

pub fn eth_account_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_accounts", |_, context| async move {
            let context = context.read().await;
            logic::accounts(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getTransactionCount", |params, context| async move {
            let context = context.read().await;
            let (address, block_number) = params.parse()?;
            logic::get_transaction_count(address, block_number, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getBalance", |params, context| async move {
            let context = context.read().await;
            let (address, block_number) = params.parse()?;
            logic::get_balance(address, block_number, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getCode", |params, context| async move {
            let context = context.read().await;
            let (address, block_number) = params.parse()?;
            logic::get_code(address, block_number, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getStorageAt", |params, context| async move {
            let context = context.read().await;
            let (address, key, block_number) = params.parse()?;
            logic::get_storage_at(address, key, block_number, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
