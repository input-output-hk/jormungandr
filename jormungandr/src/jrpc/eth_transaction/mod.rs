use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

pub fn eth_transaction_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_sendTransaction", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_sendRawTransaction", |params, context| async move {
            let context = context.read().await;
            let raw_tx = params.parse()?;
            logic::send_raw_transaction(raw_tx, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getTransactionByHash", |params, context| async move {
            let context = context.read().await;
            let hash = params.parse()?;
            logic::get_transaction_by_hash(hash, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method(
            "eth_getTransactionByBlockHashAndIndex",
            |params, context| async move {
                let context = context.read().await;
                let (hash, index) = params.parse()?;
                logic::get_transaction_by_block_hash_and_index(hash, index, &context)
                    .await
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method(
            "eth_getTransactionByBlockNumberAndIndex",
            |params, context| async move {
                let context = context.read().await;
                let (number, index) = params.parse()?;
                logic::get_transaction_by_block_number_and_index(number, index, &context)
                    .await
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method("eth_getTransactionReceipt", |params, context| async move {
            let context = context.read().await;
            let hash = params.parse()?;
            logic::get_transaction_receipt(hash, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_signTransaction", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::sign_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_estimateGas", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::estimate_gas(tx, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_sign", |params, context| async move {
            let context = context.read().await;
            let (address, message) = params.parse()?;
            logic::sign(address, message, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_call", |params, context| async move {
            let context = context.read().await;
            let (tx, number) = params.parse()?;
            logic::call(tx, number, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
