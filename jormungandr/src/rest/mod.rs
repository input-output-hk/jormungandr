//! REST API of the node

mod server;

pub mod v0;

pub use self::server::{Error, Server};

use crate::blockchain::BlockchainR;
use crate::fragment::Logs;
use crate::settings::start::{Error as ConfigError, Rest};
use std::sync::{Arc, Mutex};

pub struct Context {
    pub stats_counter: v0::node::stats::StatsCounter,
    pub blockchain: BlockchainR,
    pub transaction_task: v0::message::post::Task,
    pub logs: Logs,
}

pub fn start_rest_server(config: &Rest, context: Context) -> Result<Server, ConfigError> {
    let prefix = config
        .prefix
        .as_ref()
        .map(|prefix| prefix.as_str())
        .unwrap_or("");

    Server::builder(prefix)
        .add_handler(v0::account::create_handler(context.blockchain.clone()))
        .add_handler(v0::block::create_handler(context.blockchain.clone()))
        .add_handler(v0::node::stats::create_handler(context.stats_counter))
        .add_handler(v0::tip::create_handler(context.blockchain.clone()))
        .add_handler(v0::message::post::create_handler(context.transaction_task))
        .add_handler(v0::message::logs::create_handler(Arc::new(Mutex::new(
            context.logs,
        ))))
        .add_handler(v0::utxo::create_handler(context.blockchain))
        .build(config.pkcs12.clone(), config.listen.clone())
        .map_err(|e| e.into())
}
