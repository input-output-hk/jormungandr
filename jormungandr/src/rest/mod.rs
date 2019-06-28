//! REST API of the node

mod server;

pub mod v0;

pub use self::server::{Error, Server};

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

pub fn start_rest_server(config: &Rest, context: Context) -> Result<Server, ConfigError> {
    Server::start(config.pkcs12.clone(), config.listen.clone(), move || {
        vec![v0::app(context.clone()).boxed()]
    })
    .map_err(|e| e.into())
}
