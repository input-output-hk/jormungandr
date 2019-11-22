//! REST API of the node

mod server;

pub mod explorer;
pub mod v0;

pub use self::server::{Error, Server};

use actix_web::dev::Resource;
use actix_web::error::{Error as ActixError, ErrorInternalServerError, ErrorServiceUnavailable};
use actix_web::middleware::cors::Cors;
use actix_web::App;

use futures::{Future, IntoFuture};
use slog::Logger;
use std::sync::{Arc, RwLock};

use crate::blockchain::{Blockchain, Tip};
use crate::fragment::Logs;
use crate::leadership::Logs as LeadershipLogs;
use crate::secure::enclave::Enclave;
use crate::settings::start::{Cors as CorsConfig, Error as ConfigError, Rest};
use crate::stats_counter::StatsCounter;

use crate::intercom::{NetworkMsg, TransactionMsg};
use crate::utils::async_msg::MessageBox;

use jormungandr_lib::interfaces::NodeState;

#[derive(Clone)]
pub struct Context {
    full: Arc<RwLock<Option<Arc<FullContext>>>>,
    server: Arc<RwLock<Option<Arc<Server>>>>,
    node_state: Arc<RwLock<NodeState>>,
    logger: Arc<RwLock<Option<Logger>>>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            full: Default::default(),
            server: Default::default(),
            node_state: Arc::new(RwLock::new(NodeState::StartingRestServer)),
            logger: Default::default(),
        }
    }

    pub fn set_full(&self, full_context: FullContext) {
        *self.full.write().expect("Context state poisoned") = Some(Arc::new(full_context));
    }

    pub fn try_full_fut(&self) -> impl Future<Item = Arc<FullContext>, Error = ActixError> {
        self.try_full().into_future()
    }

    pub fn try_full(&self) -> Result<Arc<FullContext>, ActixError> {
        self.full
            .read()
            .expect("Context state poisoned")
            .clone()
            .ok_or_else(|| ErrorServiceUnavailable("Full REST context not available yet"))
    }

    fn set_server(&self, server: Server) {
        *self.server.write().expect("Context server poisoned") = Some(Arc::new(server));
    }

    pub fn server(&self) -> Result<Arc<Server>, ActixError> {
        self.server
            .read()
            .expect("Context server poisoned")
            .clone()
            .ok_or_else(|| ErrorInternalServerError("Server not set in  REST context"))
    }

    pub fn set_node_state(&self, node_state: NodeState) {
        *self
            .node_state
            .write()
            .expect("Context node state poisoned") = node_state;
    }

    pub fn node_state(&self) -> NodeState {
        self.node_state
            .read()
            .expect("Context node state poisoned")
            .clone()
    }

    pub fn set_logger(&self, logger: Logger) {
        *self.logger.write().expect("Context logger poisoned") = Some(logger);
    }

    pub fn logger(&self) -> Result<Logger, ActixError> {
        self.logger
            .read()
            .expect("Context logger poisoned")
            .clone()
            .ok_or_else(|| ErrorInternalServerError("Logger not set in  REST context"))
    }
}

#[derive(Clone)]
pub struct FullContext {
    pub stats_counter: StatsCounter,
    pub blockchain: Blockchain,
    pub blockchain_tip: Tip,
    pub network_task: MessageBox<NetworkMsg>,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub logs: Logs,
    pub leadership_logs: LeadershipLogs,
    pub enclave: Enclave,
    pub explorer: Option<crate::explorer::Explorer>,
}

pub fn run_rest_server(
    config: Rest,
    explorer_enabled: bool,
    context: Context,
) -> Result<(), ConfigError> {
    let app_context = context.clone();
    let cors_cfg = config.cors;
    let handlers = move || {
        let mut apps = vec![build_app(
            app_context.clone(),
            "/api/v0",
            v0::resources(),
            &cors_cfg,
        )];

        if explorer_enabled {
            apps.push(build_app(
                app_context.clone(),
                "/explorer",
                explorer::resources(),
                &cors_cfg,
            ))
        }

        apps
    };
    let server_receiver = move |server| context.set_server(server);
    Server::run(config.pkcs12, config.listen, handlers, server_receiver).map_err(Into::into)
}

fn build_app<S, P, R>(state: S, prefix: P, resources: R, cors_cfg: &Option<CorsConfig>) -> App<S>
where
    S: 'static,
    P: Into<String>,
    R: IntoIterator<Item = (&'static str, &'static dyn Fn(&mut Resource<S>))>,
{
    let app = App::with_state(state).prefix(prefix);
    match cors_cfg {
        Some(cors_cfg) => register_resources_with_cors(app, resources, cors_cfg),
        None => register_resources(app, resources),
    }
}

fn register_resources<S, R>(mut app: App<S>, resources: R) -> App<S>
where
    S: 'static,
    R: IntoIterator<Item = (&'static str, &'static dyn Fn(&mut Resource<S>))>,
{
    for (path, resource) in resources {
        app = app.resource(path, resource);
    }
    app
}

fn register_resources_with_cors<S, R>(app: App<S>, resources: R, cors_cfg: &CorsConfig) -> App<S>
where
    S: 'static,
    R: IntoIterator<Item = (&'static str, &'static dyn Fn(&mut Resource<S>))>,
{
    let mut cors = Cors::for_app(app);
    if let Some(max_age_secs) = cors_cfg.max_age_secs {
        cors.max_age(max_age_secs as usize);
    }
    for origin in &cors_cfg.allowed_origins {
        cors.allowed_origin(origin);
    }
    for (path, resource) in resources {
        cors.resource(path, resource);
    }
    cors.register()
}
