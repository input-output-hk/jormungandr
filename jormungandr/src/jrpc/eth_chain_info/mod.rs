use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

pub fn eth_chain_info_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_chainId", |_, context| async move {
            let context = context.read().await;
            logic::chain_id(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_syncing", |_, context| async move {
            let context = context.read().await;
            logic::syncing(&context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_gasPrice", |_, context| async move {
            let context = context.read().await;
            logic::gas_price(&context)
                .await
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_protocolVersion", |_, context| async move {
            let context = context.read().await;
            logic::protocol_verion(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_feeHistory", |params, context| async move {
            let context = context.read().await;
            let (block_count, newest_block, reward_percentiles) = params.parse()?;
            logic::fee_history(block_count, newest_block, reward_percentiles, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
