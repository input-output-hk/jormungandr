use actix_net::server::Server;
use actix_web::actix::{Addr, MailboxError, System};
use actix_web::server;
use actix_web::server::{IntoHttpHandler, StopServer};
use futures::Future;
use native_tls::{Identity, TlsAcceptor};
use rest::server_service::{Error, ServerResult, ServerServiceBuilder};
use std::fs;
use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::sync::mpsc::sync_channel;
use std::thread;

#[derive(Clone)]
pub struct ServerService {
    addr: Addr<Server>,
}

impl ServerService {
    pub fn builder(
        pkcs12: impl AsRef<Path>,
        address: SocketAddr,
        prefix: impl Into<String>,
    ) -> ServerServiceBuilder {
        ServerServiceBuilder::new(pkcs12, address, prefix)
    }

    pub fn start<P, F, H>(pkcs12: P, address: SocketAddr, handler: F) -> ServerResult<Self>
    where
        P: AsRef<Path>,
        F: Fn() -> H + Send + Clone + 'static,
        H: IntoHttpHandler + 'static,
    {
        let tls = load_tls_acceptor(pkcs12)?;
        let (sender, receiver) = sync_channel::<ServerResult<ServerService>>(0);
        thread::spawn(move || {
            let actix_system = System::builder().build();
            let server_handler = start_server_curr_actix_system(address, tls, handler);
            let run_system = server_handler.is_ok();
            let _ = sender.send(server_handler);
            if run_system {
                actix_system.run();
            }
        });
        receiver.recv().unwrap()
    }

    pub fn stop(&self) -> impl Future<Item = (), Error = Error> {
        self.addr
            .send(StopServer { graceful: true })
            .then(|result| match result {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(_)) => Err(Error::ServerStopFailed),
                Err(MailboxError::Closed) => Err(Error::ServerAlreadyStopped),
                Err(MailboxError::Timeout) => Err(Error::ServerStopTimeout),
            })
    }
}

fn load_tls_acceptor(pkcs12_path: impl AsRef<Path>) -> ServerResult<TlsAcceptor> {
    let pkcs_12_data = fs::read(pkcs12_path).map_err(|e| Error::Pkcs12LoadFailed(e))?;
    let identity = Identity::from_pkcs12(&pkcs_12_data, "").map_err(|e| Error::Pkcs12Invalid(e))?;
    TlsAcceptor::new(identity).map_err(|e| Error::Pkcs12Invalid(e))
}

fn start_server_curr_actix_system<F, H>(
    address: impl ToSocketAddrs,
    tls: TlsAcceptor,
    handler: F,
) -> ServerResult<ServerService>
where
    F: Fn() -> H + Send + Clone + 'static,
    H: IntoHttpHandler + 'static,
{
    let addr = server::new(handler)
        .system_exit()
        .disable_signals()
        .bind_tls(address, tls)
        .map_err(|err| Error::BindFailed(err))?
        .start();
    Ok(ServerService { addr })
}
