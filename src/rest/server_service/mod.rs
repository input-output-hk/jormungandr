//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;
mod server_service;

pub use self::error::Error;
pub use self::server_service::ServerService;

pub type ServerResult<T> = Result<T, Error>;
