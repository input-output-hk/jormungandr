//! REST API of the node

mod server_service;

pub use self::server_service::{Error, ServerService};

use settings::Error as SettingsError;
use settings::start::{Error as ConfigError, Rest};

use actix_web::App;

pub fn start_rest_server(config: &Rest) -> Result<ServerService, SettingsError> {
    let handler = move || {
        App::with_state(()).prefix("api").scope("v1", |scope| {
            scope.resource("/hello", |r| {
                r.get().with(|_: ()| {
                    println!("HELLO");
                    "Hello world"
                })
            })
        })
    };
    ServerService::start(&config.pkcs12, config.listen.clone(), handler)
        .map_err(|e| SettingsError::Start(ConfigError::InvalidRest(e)))
}
