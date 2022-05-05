use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_filter_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_newFilter", |_, context| async move {
            let context = context.read().await;
            logic::new_filter(&context)
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
        .register_async_method("eth_uninstallFilter", |_, context| async move {
            let context = context.read().await;
            logic::uninstall_filter(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getFilterChanges", |_, context| async move {
            let context = context.read().await;
            logic::get_filter_changes(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getFilterLogs", |_, context| async move {
            let context = context.read().await;
            logic::get_filter_logs(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();
    module
}
