//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

mod grpc;
// TODO: to be ported
//mod ntt;
mod service;

use std::{sync::Arc, time::Duration};

use crate::blockcfg::BlockConfig;
use crate::blockchain::BlockchainR;
use crate::intercom::{BlockMsg, ClientMsg, NetworkBroadcastMsg, TransactionMsg};
use crate::settings::network::{Configuration, Connection, Listen, Peer, Protocol};
use crate::utils::task::TaskMessageBox;

use chain_core::property;
use futures::prelude::*;
use futures::{
    future,
    stream::{self, Stream},
    sync::mpsc,
};

/// all the different channels the network may need to talk to
pub struct Channels<B: BlockConfig> {
    pub client_box: TaskMessageBox<ClientMsg<B>>,
    pub transaction_box: TaskMessageBox<TransactionMsg<B>>,
    pub block_box: TaskMessageBox<BlockMsg<B>>,
}

impl<B: BlockConfig> Clone for Channels<B> {
    fn clone(&self) -> Self {
        Channels {
            client_box: self.client_box.clone(),
            transaction_box: self.transaction_box.clone(),
            block_box: self.block_box.clone(),
        }
    }
}

pub struct GlobalState<B: BlockConfig> {
    pub config: Arc<Configuration>,
    pub channels: Channels<B>,
}

impl<B: BlockConfig> Clone for GlobalState<B> {
    fn clone(&self) -> Self {
        GlobalState {
            config: self.config.clone(),
            channels: self.channels.clone(),
        }
    }
}

pub struct ConnectionState<B: BlockConfig> {
    /// The global network configuration
    pub global_network_configuration: Arc<Configuration>,

    /// the channels the connection will need to have to
    /// send messages too
    pub channels: Channels<B>,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    /// the local (to the task) connection details
    pub connection: Connection,

    pub connected: Option<Connection>,
}

impl<B: BlockConfig> Clone for ConnectionState<B> {
    fn clone(&self) -> Self {
        ConnectionState {
            global_network_configuration: self.global_network_configuration.clone(),
            channels: self.channels.clone(),
            timeout: self.timeout,
            connection: self.connection.clone(),
            connected: self.connected.clone(),
        }
    }
}

impl<B: BlockConfig> ConnectionState<B> {
    fn new_listen(global: &GlobalState<B>, listen: Listen) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: listen.timeout,
            connection: listen.connection,
            connected: None,
        }
    }

    fn new_peer(global: &GlobalState<B>, peer: Peer) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: peer.timeout,
            connection: peer.connection,
            connected: None,
        }
    }
    fn connected(mut self, connection: Connection) -> Self {
        self.connected = Some(connection);
        self
    }
}

pub fn run<B>(
    config: Configuration,
    subscription_msg_box: mpsc::UnboundedReceiver<NetworkBroadcastMsg<B>>, // TODO: abstract away Cardano
    channels: Channels<B>,
) where
    B: BlockConfig + 'static,
{
    let arc_config = Arc::new(config.clone());
    let state = GlobalState {
        config: arc_config,
        channels: channels,
    };

    let state_listener = state.clone();
    // open the port for listening/accepting other peers to connect too
    let listener =
        stream::iter_ok(config.listen_to).for_each(move |listen| match listen.connection {
            Connection::Tcp(sockaddr) => match listen.protocol {
                Protocol::Grpc => grpc::run_listen_socket(sockaddr, listen, state_listener.clone()),
                Protocol::Ntt => unimplemented!(), // ntt::run_listen_socket(sockaddr, listen, state_listener.clone()),
            },
            #[cfg(unix)]
            Connection::Unix(_path) => unimplemented!(),
        });

    let state_connection = state.clone();
    let connections =
        stream::iter_ok(config.peer_nodes).for_each(move |peer| match peer.connection {
            Connection::Tcp(sockaddr) => match peer.protocol {
                Protocol::Ntt => {
                    unimplemented!(); // ntt::run_connect_socket(sockaddr, peer, state_connection.clone()),
                    future::ok(())
                }
                Protocol::Grpc => unimplemented!(),
            },
            #[cfg(unix)]
            Connection::Unix(_path) => unimplemented!(),
        });

    tokio::run(connections.join(listener).map(|_| ()));
}

pub fn bootstrap<B>(config: &Configuration, blockchain: BlockchainR<B>)
where
    B: BlockConfig,
    <B::Ledger as property::Ledger>::Update: Clone,
    <B::Settings as property::Settings>::Update: Clone,
    <B::Leader as property::LeaderSelection>::Update: Clone,
    for<'a> &'a <B::Block as property::HasTransaction>::Transactions:
        IntoIterator<Item = &'a B::Transaction>,
{
    let grpc_peer = config
        .peer_nodes
        .iter()
        .filter(|peer| peer.protocol == Protocol::Grpc)
        .next();
    match grpc_peer {
        Some(peer) => grpc::bootstrap_from_peer(peer.clone(), blockchain),
        None => {
            warn!("no gRPC peers specified, skipping bootstrap");
        }
    }
}
