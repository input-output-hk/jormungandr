extern crate chain_core;
extern crate prost;
extern crate tower_grpc;
extern crate tower_h2;

// Generated protobuf/gRPC code.
#[allow(dead_code)]
mod gen {
    pub mod node {
        include!(concat!(env!("OUT_DIR"), "/iohk.chain.node.rs"));
    }
}

pub mod client;
pub mod peer;
pub mod server;
mod service;
