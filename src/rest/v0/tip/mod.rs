use crate::{blockcfg::mock::Mockchain, blockchain::BlockchainR};
use actix_web::{App, Responder, State};
use chain_core::property::Settings as _;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        App::with_state(blockchain.clone())
            .prefix(format!("{}/v0/tip", prefix))
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(settings: State<BlockchainR<Mockchain>>) -> impl Responder {
    settings.read().unwrap().state.tip().to_string()
}
