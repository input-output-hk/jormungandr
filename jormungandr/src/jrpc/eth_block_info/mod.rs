use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;
pub use logic::get_block_by_number_from_context;

mod logic;

pub fn eth_block_info_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_getBlockByHash", |params, context| async move {
            let context = context.read().await;
            let (block_hash, full) = params.parse()?;
            logic::get_block_by_hash(block_hash, full, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getBlockByNumber", |params, context| async move {
            let context = context.read().await;
            let (block_number, full) = params.parse()?;
            logic::get_block_by_number(block_number, full, &context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method(
            "eth_getBlockTransactionCountByHash",
            |params, context| async move {
                let context = context.read().await;
                let block_hash = params.parse()?;
                logic::get_transaction_count_by_hash(block_hash, &context)
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method(
            "eth_getBlockTransactionCountByNumber",
            |params, context| async move {
                let context = context.read().await;
                let block_number = params.parse()?;
                logic::get_transaction_count_by_number(block_number, &context)
                    .await
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method(
            "eth_getUncleCountByBlockHash",
            |params, context| async move {
                let context = context.read().await;
                let block_hash = params.parse()?;
                logic::get_uncle_count_by_hash(block_hash, &context)
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method(
            "eth_getUncleCountByBlockNumber",
            |params, context| async move {
                let context = context.read().await;
                let block_number = params.parse()?;
                logic::get_uncle_count_by_number(block_number, &context)
                    .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
            },
        )
        .unwrap();

    module
        .register_async_method("eth_blockNumber", |_, context| async move {
            let context = context.read().await;
            logic::get_block_number(&context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
