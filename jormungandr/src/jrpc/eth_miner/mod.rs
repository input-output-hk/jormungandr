use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub fn eth_miner_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_mining", |_, context| async move {
            let context = context.read().await;
            logic::mining(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_hashrate", |_, context| async move {
            let context = context.read().await;
            logic::hashrate(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_getWork", |_, context| async move {
            let context = context.read().await;
            logic::get_work(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_submitWork", |_, context| async move {
            let context = context.read().await;
            logic::submit_work(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_submitHashrate", |_, context| async move {
            let context = context.read().await;
            logic::submit_hashrate(&context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();
    module
}
