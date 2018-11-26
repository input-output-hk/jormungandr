//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

mod ntt;
mod grpc;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use protocol::{
    Message,
    MessageType,
    Response,
    network_transport::LightWeightConnectionId,
};
use intercom::{ClientMsg, TransactionMsg, BlockMsg, NetworkBroadcastMsg};
use utils::task::{TaskMessageBox};
use settings::network::{self, Peer, Listen};

use futures::prelude::*;
use futures::{
    stream::{self, Stream},
    sync::mpsc,
};

/// all the different channels the network may need to talk to
#[derive(Clone)]
pub struct Channels {
    pub client_box:      TaskMessageBox<ClientMsg>,
    pub transaction_box: TaskMessageBox<TransactionMsg>,
    pub block_box:       TaskMessageBox<BlockMsg>,
}

/// Identifier to manage subscriptions
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SubscriptionId(network::Connection, LightWeightConnectionId);

/// all the subscriptions
pub type Subscriptions = HashMap<SubscriptionId, mpsc::UnboundedSender<Message>>;

pub type SubscriptionsR = Arc<RwLock<Subscriptions>>;

#[derive(Clone)]
pub struct GlobalState {
    pub config:   Arc<network::Configuration>,
    pub channels: Channels,
    pub subscriptions: SubscriptionsR,
}

#[derive(Clone)]
pub struct ConnectionState {
    /// The global network configuration
    pub global_network_configuration: Arc<network::Configuration>,

    /// the channels the connection will need to have to
    /// send messages too
    pub channels: Channels,

    /// the timeout to wait for unbefore the connection replies
    pub timeout: Duration,

    pub subscriptions: SubscriptionsR,

    /// the local (to the task) connection details
    pub connection: network::Connection,

    pub connected: Option<network::Connection>,
}
impl ConnectionState {
    fn new_listen(global: &GlobalState, listen: Listen) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: listen.timeout,
            subscriptions: global.subscriptions.clone(),
            connection: listen.connection,
            connected: None,
        }
    }
    fn new_peer(global: &GlobalState, peer: Peer) -> Self {
        ConnectionState {
            global_network_configuration: global.config.clone(),
            channels: global.channels.clone(),
            timeout: peer.timeout,
            subscriptions: global.subscriptions.clone(),
            connection: peer.connection,
            connected: None,
        }
    }
    fn connected(mut self, connection: network::Connection) -> Self {
        self.connected = Some(connection);
        self
    }
}

pub fn run( config: network::Configuration
          , subscription_msg_box: mpsc::UnboundedReceiver<NetworkBroadcastMsg>
          , channels: Channels
          )
{
    let arc_config = Arc::new(config.clone());
    let subscriptions = Arc::new(RwLock::new(Subscriptions::default()));
    let state = GlobalState {
        config:   arc_config,
        channels: channels,
        subscriptions: subscriptions.clone(),
    };

    let subscriptions = subscription_msg_box.fold(subscriptions, |subscriptions, msg| {
        info!("Sending a subscription message");
        debug!("subscription message is : {:#?}", msg);

        let subscriptions_clone = Arc::clone(&subscriptions);
        let subscriptions_err = Arc::clone(&subscriptions);

        // clone the subscribed ends into a temporary array, the RwLock will
        // lock the variable only the time to build this collection, so we should
        // be able to free it for the error handling part
        let subscriptions_col : Vec<_>
            = subscriptions_clone.read()
                                 .unwrap()
                                 .iter()
                                 .map(|(k, v)| (k.clone(), v.clone()))
                                 .collect();


        // create a stream of all the subscriptions, for every broadcast
        // messages we will send the message to broadcast
        //
        futures::stream::iter_ok::<_, ()>(subscriptions_col)
            .for_each(move |(identifier, sink)| {
                let msg = match msg.clone() {
                    NetworkBroadcastMsg::Block(block) => {
                        Message::Bytes(
                            identifier.1,
                            MessageType::MsgBlock.encode_with(
                                &Response::Ok::<_, String>(block)
                            )
                        )
                    }
                    NetworkBroadcastMsg::Header(header) => {
                        Message::Bytes(
                            identifier.1,
                            MessageType::MsgHeaders.encode_with(
                                &Response::Ok::<_, String>(cardano::block::BlockHeaders(vec![header]))
                            )
                        )
                    }
                    NetworkBroadcastMsg::Transaction(transaction) => {
                        Message::Bytes(identifier.1, cbor!(transaction).unwrap().into())
                    }
                };
                sink.unbounded_send(msg)
                    .map(|_| ())
                    .map_err(|_| {
                        // in case of an error we can remove the element from the subscription
                        //
                        // This is because the only reason the subscription would fail to be sent
                        // is if the other end of the unbound channel is disconnected/dropped.
                        // So we can assume the other end *is* disconnected.
                        //
                        // TODO: we might want to double check that and actually tell the
                        //       other end that an error occurred or at least force-drop the
                        //       connection so the client need to reconnect.
                        warn!("Subscription for {:?} failed, removing subscription...", identifier);
                        let mut subscriptions_write = subscriptions_err.write().unwrap();
                        subscriptions_write.remove(&identifier);
                    })
            }).map(|_| subscriptions)
    });

    let state_listener = state.clone();
    // open the port for listening/accepting other peers to connect too
    let listener = stream::iter_ok(config.listen_to).for_each(move |listen| {
        match listen.connection {
            network::Connection::Tcp(sockaddr) => {
                match listen.protocol {
                    network::Protocol::Ntt => ntt::run_listen_socket(
                        sockaddr, listen, state_listener.clone()
                    ),
                    network::Protocol::Grpc => grpc::run_listen_socket(
                        sockaddr, listen, state_listener.clone()
                    ),
                }
            }
            #[cfg(unix)]
            network::Connection::Unix(_path) => unimplemented!()
        }
    });

    let state_connection = state.clone();
    let connections = stream::iter_ok(config.peer_nodes).for_each(move |peer| {
        match peer.connection {
            network::Connection::Tcp(sockaddr) => {
                match peer.protocol {
                    network::Protocol::Ntt => ntt::run_connect_socket(
                        sockaddr, peer, state_connection.clone()
                    ),
                    network::Protocol::Grpc => unimplemented!(),
                }
            },
            #[cfg(unix)]
            network::Connection::Unix(_path) => unimplemented!()
        }
    });

    tokio::run(subscriptions.join3(connections, listener).map(|_| ()));
}
