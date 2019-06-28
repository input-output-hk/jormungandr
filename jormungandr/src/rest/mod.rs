//! REST API of the node

mod server;

pub mod v0;

pub use self::server::{Error, Server};


use actix_web::App;
use std::sync::{Arc, Mutex};

use crate::blockchain::BlockchainR;
use crate::fragment::Logs;
use crate::settings::start::{Error as ConfigError, Rest};
use crate::stats_counter::StatsCounter;

use crate::intercom::TransactionMsg;
use crate::utils::async_msg::MessageBox;

#[derive(Clone)]
pub struct Context {
    pub stats_counter: StatsCounter,
    pub blockchain: BlockchainR,
    pub transaction_task: Arc<Mutex<MessageBox<TransactionMsg>>>,
    pub logs: Arc<Mutex<Logs>>,
}

pub fn v0_app(context: Context) -> App<Context> {
    App::with_state(context)
        .prefix("/api/v0")
        .resource("/account/{account_id}", |r| {
            r.get().with(v0::handlers::get_account_state)
        })
        .resource("/block/{block_id}", |r| {
            r.get().with(v0::handlers::get_block_id)
        })
        .resource("/block/{block_id}/next_id", |r| {
            r.get().with(v0::handlers::get_block_next_id)
        })
        .resource("/fragment/logs", |r| {
            r.get().with(v0::handlers::get_message_logs)
        })
        .resource("/message", |r| r.post().a(v0::handlers::post_message))
        .resource("/node/stats", |r| {
            r.get().with(v0::handlers::get_stats_counter)
        })
        .resource("/tip", |r| r.get().with(v0::handlers::get_tip))
        .resource("/utxo", |r| r.get().with(v0::handlers::get_utxos))
}

pub fn start_rest_server(config: &Rest, context: Context) -> Result<Server, ConfigError> {
    Server::start(config.pkcs12.clone(), config.listen.clone(), move || {
        vec![v0_app(context.clone()).boxed()]
    })
    .map_err(|e| e.into())
}
