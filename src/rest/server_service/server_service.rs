use actix_net::server::Server;
use actix_web::actix::{Addr, MailboxError, System};
use actix_web::server;
use actix_web::server::{IntoHttpHandler, StopServer};
use futures::Future;
use native_tls::TlsAcceptor;
use rest::server_service::{Error, ServerResult};
use std::net::ToSocketAddrs;
use std::sync::mpsc::sync_channel;
use std::thread;

#[derive(Clone)]
pub struct ServerService {
    addr: Addr<Server>,
}

impl ServerService {
    pub fn start<A, F, H>(address: A, tls: TlsAcceptor, handler: F) -> ServerResult<Self>
    where
        A: ToSocketAddrs + Send + 'static,
        F: Fn() -> H + Send + Clone + 'static,
        H: IntoHttpHandler + 'static,
    {
        let (sender, receiver) = sync_channel::<ServerResult<ServerService>>(0);
        thread::spawn(move || {
            let actix_system = System::builder().build();
            let server_handler = start_server_curr_actix_system(address, tls, handler);
            let _ = sender.send(server_handler);
            actix_system.run();
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
        .bind_tls(address, tls)
        .map_err(|err| Error::BindFailed(err))?
        .start();
    Ok(ServerService { addr })
}
