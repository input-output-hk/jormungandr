//! Framework for REST API server. It's a wrapper around Actix-web allowing it
//! to be run as a background service.

mod error;

pub use self::error::Error;

use actix_net::server::Server as ActixServer;
use actix_web::{
    actix::{Addr, System},
    server::{self, IntoHttpHandler, StopServer},
};
use native_tls::{Identity, TlsAcceptor};
use std::{
    fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
};

pub type ServerResult<T> = Result<T, Error>;

#[derive(Clone)]
pub struct Server {
    addr: Addr<ActixServer>,
}

impl Server {
    pub fn run<F, H>(
        pkcs12: Option<PathBuf>,
        address: SocketAddr,
        handler: F,
        server_receiver: impl FnOnce(Server),
    ) -> ServerResult<()>
    where
        F: Fn() -> H + Clone + Send + 'static,
        H: IntoHttpHandler + 'static,
    {
        let tls = load_tls_acceptor(pkcs12)?;
        let actix_system = System::builder().build();
        let addr = start_server_curr_actix_system(address, tls, handler)?;
        let server = Server { addr };
        server_receiver(server);
        actix_system.run();
        Ok(())
    }

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
