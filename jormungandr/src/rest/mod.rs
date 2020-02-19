//! REST API of the node

mod server;

pub mod explorer;
pub mod v0;

pub use self::server::{Error, Server, ServerStopper};

use actix_web::error::{Error as ActixError, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::web::ServiceConfig;

use slog::Logger;
use std::sync::Arc;

use crate::blockchain::{Blockchain, Tip};
use crate::leadership::Logs as LeadershipLogs;
use crate::network::p2p::P2pTopology;
use crate::secure::enclave::Enclave;
use crate::settings::start::{Error as ConfigError, Rest};
use crate::stats_counter::StatsCounter;

use crate::intercom::{NetworkMsg, TransactionMsg};
use crate::utils::async_msg::MessageBox;

use futures03::executor::block_on;
use jormungandr_lib::interfaces::NodeState;
use tokio02::sync::RwLock;

#[derive(Clone)]
pub struct Context {
    full: Arc<RwLock<Option<Arc<FullContext>>>>,
    server_stopper: Arc<RwLock<Option<ServerStopper>>>,
    node_state: Arc<RwLock<NodeState>>,
    logger: Arc<RwLock<Option<Logger>>>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            full: Default::default(),
            server_stopper: Default::default(),
            node_state: Arc::new(RwLock::new(NodeState::StartingRestServer)),
            logger: Default::default(),
        }
    }

    pub async fn set_full(&self, full_context: FullContext) {
        *self.full.write().await = Some(Arc::new(full_context));
    }

    pub async fn try_full(&self) -> Result<Arc<FullContext>, ActixError> {
        self.full
            .read()
            .await
            .clone()
            .ok_or_else(|| ErrorServiceUnavailable("Full REST context not available yet"))
    }

    async fn set_server_stopper(&self, server_stopper: ServerStopper) {
        *self.server_stopper.write().await = Some(server_stopper);
    }

    pub async fn server_stopper(&self) -> Result<ServerStopper, ActixError> {
        self.server_stopper
            .read()
            .await
            .clone()
            .ok_or_else(|| ErrorInternalServerError("Server stopper not set in REST context"))
    }

    pub async fn set_node_state(&self, node_state: NodeState) {
        *self.node_state.write().await = node_state;
    }

    pub async fn node_state(&self) -> NodeState {
        self.node_state.read().await.clone()
    }

    pub async fn set_logger(&self, logger: Logger) {
        *self.logger.write().await = Some(logger);
    }

    pub async fn logger(&self) -> Result<Logger, ActixError> {
        self.logger
            .read()
            .await
            .clone()
            .ok_or_else(|| ErrorInternalServerError("Logger not set in REST context"))
    }
}

#[derive(Clone)]
pub struct FullContext {
    pub stats_counter: StatsCounter,
    pub blockchain: Blockchain,
    pub blockchain_tip: Tip,
    pub network_task: MessageBox<NetworkMsg>,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub leadership_logs: LeadershipLogs,
    pub enclave: Enclave,
    pub p2p: P2pTopology,
    pub explorer: Option<crate::explorer::Explorer>,
    pub diagnostic: crate::diagnostic::Diagnostic,
}

pub fn start_rest_server(
    config: Rest,
    explorer_enabled: bool,
    context: &Context,
) -> Result<Server, ConfigError> {
    let app_config = app_config_factory(explorer_enabled, context.clone());
    let server = Server::start(config, app_config)?;
    block_on(context.set_server_stopper(server.stopper()));
    Ok(server)
}

fn app_config_factory(
    explorer_enabled: bool,
    context: Context,
) -> impl FnOnce(&mut ServiceConfig) + Clone + Send + 'static {
    move |config| app_config(config, explorer_enabled, context)
}

fn app_config(config: &mut ServiceConfig, explorer_enabled: bool, context: Context) {
    config.data(context).service(v0::service("/api/v0"));
    if explorer_enabled {
        config.service(explorer::service("/explorer"));
    }
}
