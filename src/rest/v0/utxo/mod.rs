mod utxo;

use self::utxo::Utxo;
use actix_web::{App, Json, Responder, State};
use blockcfg::mock::Mockchain;
use blockchain::BlockchainR;

pub fn create_handler(
    blockchain: BlockchainR<Mockchain>,
) -> impl Fn(&str) -> App<BlockchainR<Mockchain>> + Send + Sync + Clone + 'static {
    move |prefix: &str| {
        let app_prefix = format!("{}/v0/utxo", prefix);
        App::with_state(blockchain.clone())
            .prefix(app_prefix)
            .resource("", |r| r.get().with(handle_request))
    }
}

fn handle_request(blockchain: State<BlockchainR<Mockchain>>) -> impl Responder {
    let utxos = blockchain
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .state
        .ledger
        .unspent_outputs
        .iter()
        .map(Utxo::from)
        .collect::<Vec<_>>();
    Json(utxos)
}
