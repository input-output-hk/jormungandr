//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;

pub use self::error::Error;

use actix_net::server::Server as ActixServer;
use actix_web::{
    actix::{Addr, System},
    server::{self, IntoHttpHandler, StopServer},
};
use futures::sync::oneshot::{self, Receiver};
use futures::{Async, Future, Poll};
use native_tls::{Identity, TlsAcceptor};
use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
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
        pkcs12: Option<PathBuf>,
        address: SocketAddr,
        handler: F,
    ) -> ServerResult<Server>
    where
        F: Fn() -> H + Clone + Send + 'static,
        H: IntoHttpHandler + 'static,
    {
        let tls = load_tls_acceptor(pkcs12)?;
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

impl Drop for Server {
    fn drop(&mut self) {
        self.stopper.stop();
        let _ = self.wait();
    }
}

impl ServerStopper {
    pub fn stop(&self) {
        self.addr.do_send(StopServer { graceful: false })
    }
}

pub fn load_tls_acceptor(pkcs12_opt: Option<PathBuf>) -> ServerResult<Option<TlsAcceptor>> {
    let pkcs12_path = match pkcs12_opt {
        Some(pkcs12) => pkcs12,
        None => return Ok(None),
    };
    let pkcs12_data = fs::read(pkcs12_path).map_err(|e| Error::Pkcs12LoadFailed(e))?;
    let identity = Identity::from_pkcs12(&pkcs12_data, "").map_err(|e| Error::Pkcs12Invalid(e))?;
    let tls = TlsAcceptor::new(identity).map_err(|e| Error::Pkcs12Invalid(e))?;
    Ok(Some(tls))
}

fn start_server_curr_actix_system<F, H>(
    address: impl ToSocketAddrs,
    tls_opt: Option<TlsAcceptor>,
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
    match tls_opt {
        Some(tls) => server.bind_tls(address, tls),
        None => server.bind(address),
    }
    .map(|bound_server| bound_server.start())
    .map_err(|err| Error::BindFailed(err))
}
