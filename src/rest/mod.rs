//! REST API of the node

mod server_service;

pub use self::server_service::{Error, ServerService};

use settings::{Error as ConfigError, Rest};

use actix_web::App;

pub fn start_rest_server(config: &Rest) -> Result<ServerService, ConfigError> {
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
        .map_err(|e| ConfigError::InvalidRest(e))
}
