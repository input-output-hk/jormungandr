//! REST API of the node

mod server;

pub mod v0;

pub use self::server::{Error, Server};

use std::sync::{Arc, Mutex, RwLock};

use crate::blockchain::{Blockchain, Branch};
use crate::fragment::Logs;
use crate::secure::enclave::Enclave;
use crate::settings::start::{Error as ConfigError, Rest};
use crate::stats_counter::StatsCounter;

use crate::intercom::TransactionMsg;
use crate::utils::async_msg::MessageBox;

#[derive(Clone)]
pub struct Context {
    pub stats_counter: StatsCounter,
    pub blockchain: Blockchain,
    pub blockchain_tip: Branch,
    pub transaction_task: Arc<Mutex<MessageBox<TransactionMsg>>>,
    pub logs: Arc<Mutex<Logs>>,
    pub server: Arc<RwLock<Option<Server>>>,
    pub enclave: Enclave,
}

pub fn start_rest_server(config: &Rest, context: Context) -> Result<Server, ConfigError> {
    let app_context = context.clone();
    let server = Server::start(config.pkcs12.clone(), config.listen.clone(), move || {
        vec![v0::app(app_context.clone()).boxed()]
    })?;
    context
        .server
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .replace(server.clone());
    Ok(server)
}
