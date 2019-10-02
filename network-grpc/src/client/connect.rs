use super::{Connection, ProtocolConfig};
use crate::gen::node::client as gen_client;

use network_core::gossip;

use futures::prelude::*;
use futures::try_ready;
use http::uri::{self, Uri};
use http_connection::HttpConnection;
use hyper::client::connect::Connect as HyperConnect;
use tower_grpc::BoxBody;
use tower_hyper::client::ConnectExecutor;
use tower_hyper::util::{Connector, Destination};
use tower_util::MakeService;

use std::{error::Error, fmt, mem};

/// Builder-like API for establishing a protocol client connection.
pub struct Connect<P, C, E>
where
    P: ProtocolConfig,
{
    tower_connect: tower_hyper::client::Connect<Destination, BoxBody, Connector<C>, E>,
    origin: Option<Origin>,
    node_id: Option<<P::Node as gossip::Node>::Id>,
}

struct Origin {
    scheme: uri::Scheme,
    authority: uri::Authority,
}

impl<P, C, E> Connect<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
    C::Transport: HttpConnection,
{
    pub fn new(connector: C, executor: E) -> Self {
        let connector = Connector::new(connector);
        let mut settings = tower_hyper::client::Builder::new();
        settings.http2_only(true);
        let tower_connect =
            tower_hyper::client::Connect::with_executor(connector, settings, executor);
        Connect {
            tower_connect,
            origin: None,
            node_id: None,
        }
    }
}

impl<P, C, E> Connect<P, C, E>
where
    P: ProtocolConfig,
{
    pub fn origin(&mut self, scheme: uri::Scheme, authority: uri::Authority) -> &mut Self {
        self.origin = Some(Origin { scheme, authority });
        self
    }

    pub fn node_id(&mut self, id: <P::Node as gossip::Node>::Id) -> &mut Self {
        self.node_id = Some(id);
        self
    }
}

impl<P, C, E> Connect<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
{
    fn origin_uri(&self, target: &Destination) -> Result<Uri, ConnectError<C::Error>> {
        let mut builder = Uri::builder();
        match self.origin {
            Some(ref origin) => {
                builder
                    .scheme(origin.scheme.clone())
                    .authority(origin.authority.clone());
            }
            None => {
                builder.scheme(target.scheme());
                let host = target.host();
                match target.port() {
                    None => {
                        builder.authority(host);
                    }
                    Some(port) => {
                        builder.authority(format!("{}:{}", host, port).as_str());
                    }
                }
            }
        };
        builder.path_and_query("");
        builder
            .build()
            .map_err(|e| ConnectError(ErrorKind::InvalidOrigin(e)))
    }
}

impl<P, C, E> Connect<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect + 'static,
    C::Transport: HttpConnection,
    E: ConnectExecutor<C::Transport, BoxBody> + Clone,
{
    pub fn connect(&mut self, target: Destination) -> ConnectFuture<P, C, E> {
        let origin_uri = match self.origin_uri(&target) {
            Ok(uri) => uri,
            Err(e) => return ConnectFuture::error(e),
        };
        let node_id = self.node_id.clone();
        let inner = self.tower_connect.make_service(target);
        ConnectFuture {
            state: State::Connecting {
                inner,
                origin_uri,
                node_id,
            },
        }
    }
}

/// Completes with a protocol client Connection when it has been
/// set up.
pub struct ConnectFuture<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
    C::Transport: HttpConnection,
{
    state: State<P, C, E>,
}

enum State<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
    C::Transport: HttpConnection,
{
    Connecting {
        inner: tower_hyper::client::ConnectFuture<Destination, BoxBody, Connector<C>, E>,
        origin_uri: Uri,
        node_id: Option<<P::Node as gossip::Node>::Id>,
    },
    Error(ConnectError<C::Error>),
    Finished,
}

impl<P, C, E> ConnectFuture<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
    C::Transport: HttpConnection,
{
    fn error(err: ConnectError<C::Error>) -> Self {
        ConnectFuture {
            state: State::Error(err),
        }
    }
}

impl<P, C, E> Future for ConnectFuture<P, C, E>
where
    P: ProtocolConfig,
    C: HyperConnect,
    C::Transport: HttpConnection,
    E: ConnectExecutor<C::Transport, BoxBody>,
{
    type Item = Connection<P>;
    type Error = ConnectError<C::Error>;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let conn_ready = if let State::Connecting { inner, .. } = &mut self.state {
            // If not connected yet, bail out here without modifying state
            Some(try_ready!(inner.poll()))
        } else {
            None
        };
        match mem::replace(&mut self.state, State::Finished) {
            State::Connecting {
                inner: _,
                origin_uri,
                node_id,
            } => {
                let conn = tower_request_modifier::Builder::new()
                    .set_origin(origin_uri)
                    .build(conn_ready.unwrap())
                    .unwrap();
                let conn = Connection {
                    service: gen_client::Node::new(conn),
                    node_id: node_id,
                };
                return Ok(Async::Ready(conn));
            }
            State::Error(e) => Err(e),
            State::Finished => panic!("polled a finished future"),
        }
    }
}

#[derive(Debug)]
pub struct ConnectError<T>(ErrorKind<T>);

#[derive(Debug)]
enum ErrorKind<T> {
    Http(tower_hyper::client::ConnectError<T>),
    InvalidOrigin(http::Error),
}

impl<T> ConnectError<T> {
    /// If the error is due to a failure to establish the transport connection,
    /// returns the underlying connection error. Otherwise, returns `None`.
    pub fn connect_error(&self) -> Option<&T> {
        use tower_hyper::client::ConnectError::*;

        if let ErrorKind::Http(Connect(e)) = &self.0 {
            Some(e)
        } else {
            None
        }
    }

    /// If the error is due to a failed HTTP/2 handshake,
    /// returns the HTTP/2 protocol error. Otherwise, returns `None`.
    pub fn http_error(&self) -> Option<&hyper::Error> {
        use tower_hyper::client::ConnectError::*;

        if let ErrorKind::Http(Handshake(e)) = &self.0 {
            Some(e)
        } else {
            None
        }
    }
}

impl<T> fmt::Display for ConnectError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ErrorKind::Http(_) => write!(f, "HTTP/2.0 connection error"),
            ErrorKind::InvalidOrigin(_) => write!(f, "invalid request origin"),
        }
    }
}

impl<T> Error for ConnectError<T>
where
    T: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self.0 {
            ErrorKind::Http(ref e) => Some(e),
            ErrorKind::InvalidOrigin(ref e) => Some(e),
        }
    }
}

impl<T> From<tower_hyper::client::ConnectError<T>> for ConnectError<T> {
    fn from(err: tower_hyper::client::ConnectError<T>) -> Self {
        ConnectError(ErrorKind::Http(err))
    }
}
