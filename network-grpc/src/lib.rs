extern crate chain_core;
#[macro_use]
extern crate futures;
extern crate prost;
extern crate tower_grpc;

// Generated protobuf/gRPC code.
#[allow(dead_code)]
mod gen {
    pub mod node {
        include!(concat!(env!("OUT_DIR"), "/iohk.chain.node.rs"));
    }
}

pub mod client;
mod convert;
pub mod server;
mod service;

pub const PROTOCOL_VERSION: u32 = 0;
