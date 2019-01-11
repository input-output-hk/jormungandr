extern crate chain_core;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_connect;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_util;

// Generated protobuf/gRPC code.
#[allow(dead_code)]
mod gen {
    use network_core::codes;

    pub mod node {
        include!(concat!(env!("OUT_DIR"), "/iohk.chain.node.rs"));
    }
}

pub mod server;
