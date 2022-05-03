use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_get_blocks_info_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_chainId", |_, context| async move {
            let context = context.read().await;
            logic::get_chain_id(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_syncing", |_, context| async move {
            let context = context.read().await;
            logic::is_syncing(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_gasPrice", |_, context| async move {
            let context = context.read().await;
            logic::get_gas_price(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_protocolVersion", |_, context| async move {
            let context = context.read().await;
            logic::get_protocol_verion(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
}
