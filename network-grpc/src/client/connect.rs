use super::{Connection, ProtocolConfig};
use crate::gen::node::client as gen_client;

use network_core::gossip;

use futures::future::Executor;
use futures::prelude::*;
use futures::try_ready;
use http::uri::{self, Uri};
use tower_grpc::BoxBody;
use tower_h2::client::Background;
use tower_util::{MakeConnection, MakeService};

use std::{error::Error, fmt, mem};

/// Builder-like API for establishing a protocol client connection.
pub struct Connect<P, A, C, E>
where
    P: ProtocolConfig,
{
    tower_connect: tower_h2::client::Connect<A, C, E, BoxBody>,
    origin: Option<Origin>,
    node_id: Option<<P::Node as gossip::Node>::Id>,
}

struct Origin {
    scheme: uri::Scheme,
    authority: uri::Authority,
}

impl<P, A, C, E> Connect<P, A, C, E>
where
    P: ProtocolConfig,
    C: MakeConnection<A> + 'static,
    E: Executor<Background<C::Connection, BoxBody>> + Clone,
{
    pub fn new(make_conn: C, executor: E) -> Self {
        Connect {
            tower_connect: tower_h2::client::Connect::new(make_conn, Default::default(), executor),
            origin: None,
            node_id: None,
        }
    }

    pub fn origin(&mut self, scheme: uri::Scheme, authority: uri::Authority) -> &mut Self {
        self.origin = Some(Origin { scheme, authority });
        self
    }

    pub fn node_id(&mut self, id: <P::Node as gossip::Node>::Id) -> &mut Self {
        self.node_id = Some(id);
        self
    }

    pub fn connect(&mut self, target: A) -> ConnectFuture<P, A, C, E> {
        let origin_uri = match self.origin {
            Some(ref origin) => {
                match Uri::builder()
                    .scheme(origin.scheme.clone())
                    .authority(origin.authority.clone())
                    .path_and_query("")
                    .build()
                {
                    Ok(uri) => uri,
                    Err(e) => {
                        return ConnectFuture::error(ConnectError(ErrorKind::InvalidOrigin(e)));
                    }
                }
            }
            None => {
                return ConnectFuture::error(ConnectError(ErrorKind::OriginMissing));
            }
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
pub struct ConnectFuture<P, A, C, E>
where
    P: ProtocolConfig,
    C: MakeConnection<A>,
{
    state: State<P, A, C, E>,
}

enum State<P, A, C, E>
where
    P: ProtocolConfig,
    C: MakeConnection<A>,
{
    Connecting {
        inner: tower_h2::client::ConnectFuture<A, C, E, BoxBody>,
        origin_uri: Uri,
        node_id: Option<<P::Node as gossip::Node>::Id>,
    },
    Error(ConnectError<C::Error>),
    Finished,
}

impl<P, A, C, E> ConnectFuture<P, A, C, E>
where
    P: ProtocolConfig,
    C: MakeConnection<A>,
{
    fn error(err: ConnectError<C::Error>) -> Self {
        ConnectFuture {
            state: State::Error(err),
        }
    }
}

impl<P, A, C, E> Future for ConnectFuture<P, A, C, E>
where
    P: ProtocolConfig,
    C: MakeConnection<A>,
    E: Executor<Background<C::Connection, BoxBody>> + Clone,
{
    type Item = Connection<P, C::Connection, E>;
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
    Http(tower_h2::client::ConnectError<T>),
    OriginMissing,
    InvalidOrigin(http::Error),
}

impl<T> fmt::Display for ConnectError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            ErrorKind::Http(_) => write!(f, "HTTP/2.0 connection error"),
            ErrorKind::OriginMissing => write!(f, "request origin not specified"),
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
            ErrorKind::OriginMissing => None,
            ErrorKind::InvalidOrigin(ref e) => Some(e),
        }
    }
}

impl<T> From<tower_h2::client::ConnectError<T>> for ConnectError<T> {
    fn from(err: tower_h2::client::ConnectError<T>) -> Self {
        ConnectError(ErrorKind::Http(err))
    }
}
