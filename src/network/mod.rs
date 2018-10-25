//! all the network related actions and processes
//!
//! This module only provides and handle the different connections
//! and act as message passing between the other modules (blockchain,
//! transactions...);
//!

use std::net::{SocketAddr};

use utils::task::{TaskMessageBox};
use settings::network::{self, Peer, Listen};

type TODO = u32;

pub struct Channels {
    client_box:      TaskMessageBox<TODO>,
    transaction_box: TaskMessageBox<TODO>,
    block_box:       TaskMessageBox<TODO>,
}

pub struct State {
    pub config: network::Configuration,
    pub channels: Channels,
}
