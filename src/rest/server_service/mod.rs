//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;
mod path_predicate;
mod server_service;
mod server_service_builder;

pub use self::error::Error;
pub use self::path_predicate::PathPredicate;
pub use self::server_service::ServerService;
pub use self::server_service_builder::ServerServiceBuilder;

pub type ServerResult<T> = Result<T, Error>;
