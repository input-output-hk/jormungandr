//! REST API of the node

mod server;

pub mod v0;

pub use self::server::{Error, Server};

use actix_web::dev::Resource;
use actix_web::middleware::cors::Cors;
use actix_web::App;
use futures::{future, Future};
use std::convert::Infallible;
use tokio::sync::lock::Lock;

use crate::blockchain::{Blockchain, Branch};
use crate::fragment::Logs;
use crate::leadership::Logs as LeadershipLogs;
use crate::secure::enclave::Enclave;
use crate::settings::start::{Cors as CorsConfig, Error as ConfigError, Rest};
use crate::stats_counter::StatsCounter;

use crate::intercom::TransactionMsg;
use crate::utils::async_msg::MessageBox;

#[derive(Clone)]
pub struct Context {
    pub stats_counter: StatsCounter,
    pub blockchain: Blockchain,
    pub blockchain_tip: Branch,
    pub transaction_task: MessageBox<TransactionMsg>,
    pub logs: Logs,
    pub leadership_logs: LeadershipLogs,
    pub server: Lock<Option<Server>>,
    pub enclave: Enclave,
    pub explorer: Option<crate::explorer::Process>,
}

pub fn start_rest_server(config: &Rest, mut context: Context) -> Result<Server, ConfigError> {
    let app_context = context.clone();
    let cors_cfg = config.cors.clone();
    let server = Server::start(config.pkcs12.clone(), config.listen.clone(), move || {
        vec![build_app(
            app_context.clone(),
            "/api/v0",
            v0::resources(),
            &cors_cfg,
        )]
    })?;
    future::poll_fn(|| Ok(context.server.poll_lock()))
        .wait()
        .unwrap_or_else(|e: Infallible| match e {})
        .replace(server.clone());
    Ok(server)
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
