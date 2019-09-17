//! Abstractions for the client-side network interface of a blockchain node.

pub mod block;
pub mod fragment;
pub mod gossip;
pub mod p2p;

use crate::error::Error;

use futures::prelude::*;
use futures::try_ready;

/// Basic workings of client connections.
pub trait Client: Sized {
    /// Poll whether this client connection is ready to send another request.
    ///
    /// The users should make sure the client is ready before sending
    /// service requests.
    fn poll_ready(&mut self) -> Poll<(), Error>;

    /// Get a `Future` of when this client connection is ready to send
    /// another request.
    fn ready(self) -> ClientReady<Self> {
        ClientReady::new(self)
    }
}

/// Future that resolves to the client connection object when it is ready
/// to send another request.
pub struct ClientReady<C> {
    client: Option<C>,
}

impl<C> ClientReady<C> {
    fn new(client: C) -> Self {
        ClientReady {
            client: Some(client),
        }
    }
}

impl<C: Client> Future for ClientReady<C> {
    type Item = C;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let client = self.client.as_mut().expect("polled a finished future");
        try_ready!(client.poll_ready());
        Ok(Async::Ready(self.client.take().unwrap()))
    }
}
