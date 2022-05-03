use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_transaction_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_sendTransaction", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_sendRawTransaction", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getTransactionByHash", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method(
            "eth_getTransactionByBlockHashAndIndex",
            |params, context| async move {
                let context = context.read().await;
                let tx = params.parse()?;
                logic::send_transaction(tx, &context)
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method(
            "eth_getTransactionByBlockNumberAndIndex",
            |params, context| async move {
                let context = context.read().await;
                let tx = params.parse()?;
                logic::send_transaction(tx, &context)
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method("eth_getTransactionReceipt", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_sign", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_signTransaction", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_estimateGas", |params, context| async move {
            let context = context.read().await;
            let tx = params.parse()?;
            logic::send_transaction(tx, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
