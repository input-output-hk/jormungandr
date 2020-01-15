//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;

pub use self::error::Error;

use crate::settings::start::Tls as TlsConfig;
use actix_net::server::Server as ActixServer;
use actix_web::{
    actix::{Addr, System},
    server::{self, IntoHttpHandler, StopServer},
};
use futures::sync::oneshot::{self, Receiver};
use futures::{Async, Future, Poll};
use rustls::{internal::pemfile, Certificate, NoClientAuth, PrivateKey, ServerConfig};
use std::{
    fs::File,
    io::BufReader,
    net::{SocketAddr, ToSocketAddrs},
    sync::mpsc,
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
    addr: Addr<ActixServer>,
}

impl Server {
    pub fn start<F, H>(
        tls_config: Option<TlsConfig>,
        address: SocketAddr,
        handler: F,
    ) -> ServerResult<Server>
    where
        F: Fn() -> H + Clone + Send + 'static,
        H: IntoHttpHandler + 'static,
    {
        let tls = tls_config.map(load_rustls_config).transpose()?;
        let (server_sender, server_receiver) = mpsc::sync_channel::<ServerResult<Server>>(0);
        thread::spawn(move || {
            let actix_system = System::builder().build();
            let (stop_sender, stop_receiver) = oneshot::channel();
            let server_res =
                start_server_curr_actix_system(address, tls, handler).map(move |addr| Server {
                    stopper: ServerStopper { addr },
                    stop_receiver,
                });
            let run_system = server_res.is_ok();
            let _ = server_sender.send(server_res);
            if run_system {
                actix_system.run();
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
    pub fn stop(&self) {
        self.addr.do_send(StopServer { graceful: false })
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

fn start_server_curr_actix_system<F, H>(
    address: impl ToSocketAddrs,
    tls_config_opt: Option<ServerConfig>,
    handler: F,
) -> ServerResult<Addr<ActixServer>>
where
    F: Fn() -> H + Clone + Send + 'static,
    H: IntoHttpHandler + 'static,
{
    let server = server::new(handler)
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
}
