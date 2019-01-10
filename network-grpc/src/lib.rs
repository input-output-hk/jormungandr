extern crate chain_core;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_connect;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_util;

// Included generated protobuf/gRPC code,

#[allow(dead_code)]
mod gen {
    include!(concat!(env!("OUT_DIR"), "/iohk.chain.node.rs"));
}

pub mod server;

// TODO: replace with network_core crate
mod network_core;
