use actix_web::App;
use actix_web::server::{HttpHandler, HttpHandlerTask};
use rest::server_service::{ServerResult, ServerService};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ServerServiceBuilder {
    pkcs12: PathBuf,
    address: SocketAddr,
    handlers: Vec<Box<Fn() -> Box<HttpHandler<Task = Box<HttpHandlerTask>>> + Send + Sync>>,
}

impl ServerServiceBuilder {
    pub fn new(pkcs12: impl AsRef<Path>, address: SocketAddr) -> Self {
        Self {
            pkcs12: pkcs12.as_ref().to_path_buf(),
            address,
            handlers: vec![],
        }
    }

    pub fn add_handler<F, S: 'static>(mut self, handler: F) -> Self
        where
            F: Fn() -> App<S> + Send + Sync + Clone + 'static,
    {
        let boxing_handler = move || handler().boxed();
        self.handlers.push(Box::new(boxing_handler));
        self
    }

    pub fn build(self) -> ServerResult<ServerService> {
        let handlers = Arc::new(self.handlers);
        let multi_handler = move || handlers.iter()
            .map(|handler| handler())
            .collect::<Vec<_>>();
        ServerService::start(self.pkcs12, self.address, multi_handler)
    }
}
