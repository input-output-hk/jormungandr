use actix_web::http::header;
use actix_web::middleware::cors::Cors;
use actix_web::server::{HttpHandler, HttpHandlerTask};
use actix_web::{pred, App};
use rest::server_service::{ServerResult, ServerService};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ServerServiceBuilder {
    pkcs12: Option<PathBuf>,
    address: SocketAddr,
    prefix: Arc<String>,
    handlers: Vec<Box<Fn() -> Box<HttpHandler<Task = Box<HttpHandlerTask>>> + Send + Sync>>,
}

impl ServerServiceBuilder {
    pub fn new(pkcs12: Option<PathBuf>, address: SocketAddr, prefix: impl Into<String>) -> Self {
        Self {
            pkcs12,
            address,
            prefix: Arc::new(prefix.into()),
            handlers: vec![],
        }
        .add_handler(create_options_handler())
    }

    /// Warning! App will consume every request which passes filtering and matches prefix.
    /// The consumed request will not be passed to other handlers, so make sure that app
    /// consumes only request, which are not valid for other handlers.
    /// The prefix passed to a handler function must be added on the beginning of an app prefix.
    pub fn add_handler<F, S: 'static>(mut self, handler: F) -> Self
    where
        F: Fn(&str) -> App<S> + Send + Sync + Clone + 'static,
    {
        let prefix = self.prefix.clone();
        let wrapped_handler = move || {
            handler(&*prefix)
                .middleware(create_cors_middleware())
                .boxed()
        };
        self.handlers.push(Box::new(wrapped_handler));
        self
    }

    pub fn build(self) -> ServerResult<ServerService> {
        let handlers = Arc::new(self.handlers);
        let multi_handler = move || handlers.iter().map(|handler| handler()).collect::<Vec<_>>();
        ServerService::start(self.pkcs12, self.address, multi_handler)
    }
}

fn create_options_handler() -> impl Fn(&str) -> App<()> + Send + Sync + Clone + 'static {
    |prefix| App::new().filter(pred::Options()).prefix(prefix)
}

fn create_cors_middleware() -> Cors {
    Cors::build()
        .send_wildcard()
        .allowed_header(header::CONTENT_TYPE)
        .finish()
}
