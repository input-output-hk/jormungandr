//! REST API of the node

mod server;

pub mod context;
pub mod explorer;
pub mod v0;

pub use self::{
    context::{Context, FullContext},
    server::{Error, Server, ServerStopper},
};

use crate::settings::start::{Error as ConfigError, Rest};

use actix_web::web::ServiceConfig;
use futures03::executor::block_on;

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
