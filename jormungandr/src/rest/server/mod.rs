//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;

pub use self::error::Error;

use crate::settings::start::{Cors as CorsConfig, Rest, Tls as TlsConfig};
use actix_cors::Cors;
use actix_rt::System;
use actix_web::{dev::Server as ActixServer, web::ServiceConfig, App, HttpServer};
use futures::sync::oneshot::{self, Receiver};
use futures::{Async, Future, Poll};
use rustls::{internal::pemfile, Certificate, NoClientAuth, PrivateKey, ServerConfig};
use std::{
    fs::File,
    io::BufReader,
    net::ToSocketAddrs,
    sync::{mpsc, Arc},
    thread,
};

pub type ServerResult<T> = Result<T, Error>;

/// A future that resolves when server shuts down
/// Dropping Server causes its shutdown
pub struct Server {
    stopper: ServerStopper,
    stop_receiver: Receiver<()>,
}

#[derive(Clone)]
pub struct ServerStopper {
    actix_server: ActixServer,
}

impl Server {
    pub fn start(
        rest: Rest,
        app_config: impl FnOnce(&mut ServiceConfig) + Clone + Send + 'static,
    ) -> ServerResult<Server> {
        let address = rest.listen;
        let tls = rest.tls.map(load_rustls_config).transpose()?;
        let cors = rest.cors.map(create_cors_factory);
        let (server_sender, server_receiver) = mpsc::sync_channel::<ServerResult<Server>>(0);
        thread::spawn(move || {
            let actix_system = System::builder().build();
            let (stop_sender, stop_receiver) = oneshot::channel();
            let server_res =
                start_server_curr_sys(address, tls, cors, app_config).map(move |actix_server| {
                    Server {
                        stopper: ServerStopper { actix_server },
                        stop_receiver,
                    }
                });
            let run_system = server_res.is_ok();
            let _ = server_sender.send(server_res);
            if run_system {
                let _ = actix_system.run();
            };
            let _ = stop_sender.send(());
        });
        server_receiver
            .recv()
            .expect("Actix thread terminated before sending server handle")
    }

    pub fn stopper(&self) -> ServerStopper {
        self.stopper.clone()
    }
}

impl Future for Server {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.stop_receiver.poll() {
            Err(_) => Ok(Async::Ready(())),
            Ok(ok) => Ok(ok),
        }
    }
}

impl ServerStopper {
    /// Starts server stopping routine in fire-forget fashion
    pub fn stop(&self) {
        let gracefully = false;
        let _ = self.actix_server.stop(gracefully);
    }
}

fn load_rustls_config(config: TlsConfig) -> ServerResult<ServerConfig> {
    let certs = load_certs(&config.cert_file)?;
    let priv_key = load_priv_key(&config.priv_key_file)?;
    let mut config = ServerConfig::new(NoClientAuth::new());
    config
        .set_single_cert(certs, priv_key)
        .map_err(Error::SetCertFailed)?;
    Ok(config)
}

fn load_certs(path: &str) -> ServerResult<Vec<Certificate>> {
    let file = File::open(path).map_err(Error::CertFileOpenFailed)?;
    let certs =
        pemfile::certs(&mut BufReader::new(file)).map_err(|_| Error::CertFileParsingFailed)?;
    if certs.is_empty() {
        return Err(Error::CertFileEmpty);
    }
    Ok(certs)
}

fn load_priv_key(path: &str) -> ServerResult<PrivateKey> {
    let file = File::open(path).map_err(Error::PrivKeyFileOpenFailed)?;
    let mut priv_keys = pemfile::pkcs8_private_keys(&mut BufReader::new(file))
        .map_err(|_| Error::PrivKeyFileParsingFailed)?;
    if priv_keys.len() != 1 {
        return Err(Error::PrivKeyFileKeyCountInvalid(priv_keys.len()));
    }
    Ok(priv_keys.pop().unwrap())
}

fn create_cors_factory(cors_cfg: CorsConfig) -> impl Fn() -> Cors + Clone + Send + 'static {
    let cors_cfg_shared = Arc::new(cors_cfg);
    move || create_cors(&*cors_cfg_shared)
}

fn create_cors(cors_cfg: &CorsConfig) -> Cors {
    let mut cors = Cors::new();
    if let Some(max_age_secs) = cors_cfg.max_age_secs {
        cors = cors.max_age(max_age_secs as usize);
    }
    for origin in &cors_cfg.allowed_origins {
        cors = cors.allowed_origin(origin);
    }
    cors
}

fn start_server_curr_sys(
    address: impl ToSocketAddrs,
    tls_config_opt: Option<ServerConfig>,
    cors_factory: Option<impl Fn() -> Cors + Clone + Send + 'static>,
    app_config: impl FnOnce(&mut ServiceConfig) + Clone + Send + 'static,
) -> ServerResult<ActixServer> {
    // This macro-based pseud generic is needed because addition of CORS changes server type.
    // It's not possible to use real generics because concrete server
    // type boundaries are volatile and base on private types.

    macro_rules! start_server_curr_sys {
        ($($wrapper:ident)?) => {{
            let app_factory = move ||
                App::new()
                    $(
                        .wrap($wrapper())
                    )*
                    .configure(app_config.clone());
            let server = HttpServer::new(app_factory)
                .workers(1)
                .system_exit()
                .disable_signals();
            let server = match tls_config_opt {
                Some(tls_config) => server.bind_rustls(address, tls_config),
                None => server.bind(address),
            }
            .map_err(Error::BindFailed)?;
            let server_addr = server.start();
            Ok(server_addr)
        }}
    }

    match cors_factory {
        Some(cors) => start_server_curr_sys!(cors),
        None => start_server_curr_sys!(),
    }
}
