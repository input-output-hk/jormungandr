use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_filter_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_newFilter", |params, context| async move {
            let context = context.read().await;
            let filter = params.parse()?;
            logic::new_filter(filter, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_newBlockFilter", |_, context| async move {
            let context = context.read().await;
            logic::new_block_filter(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_newPendingTransactionFilter", |_, context| async move {
            let context = context.read().await;
            logic::new_pending_transaction_filter(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_uninstallFilter", |params, context| async move {
            let context = context.read().await;
            let filter_id = params.parse()?;
            logic::uninstall_filter(filter_id, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getFilterChanges", |params, context| async move {
            let context = context.read().await;
            let filter_id = params.parse()?;
            logic::get_filter_changes(filter_id, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getFilterLogs", |params, context| async move {
            let context = context.read().await;
            let filter_id = params.parse()?;
            logic::get_filter_logs(filter_id, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getLogs", |params, context| async move {
            let context = context.read().await;
            let filter = params.parse()?;
            logic::get_logs(filter, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();
    module
}
