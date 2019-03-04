use actix_web::{App, Responder, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;
use chain_core::property::Settings;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/tip", prefix))
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(blockchain: State<BlockchainR<Mockchain>>) -> impl Responder {
    blockchain
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .state
        .settings
        .tip()
        .to_string()
}
