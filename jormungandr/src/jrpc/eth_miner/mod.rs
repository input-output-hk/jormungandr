use crate::context::ContextLock;
use jsonrpsee_http_server::RpcModule;

mod logic;

pub fn eth_miner_module(context: ContextLock) -> RpcModule<ContextLock> {
    let mut module = RpcModule::new(context);

    module
        .register_async_method("eth_mining", |_, context| async move {
            let context = context.read().await;
            logic::mining(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_coinbase", |_, context| async move {
            let context = context.read().await;
            logic::coinbase(&context).map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
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
        .register_async_method("eth_submitWork", |params, context| async move {
            let context = context.read().await;
            let (nonce, pow_hash, mix_digest) = params.parse()?;
            logic::submit_work(nonce, pow_hash, mix_digest, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();

    module
        .register_async_method("eth_submitHashrate", |params, context| async move {
            let context = context.read().await;
            let (hash_rate, id) = params.parse()?;
            logic::submit_hashrate(hash_rate, id, &context)
                .map_err(|err| jsonrpsee_core::Error::Custom(err.to_string()))
        })
        .unwrap();
    module
}
