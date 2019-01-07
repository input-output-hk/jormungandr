extern crate chain_core;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_connect;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_util;

// Included generated protobuf/gRPC code,
// namespaced into submodules corresponding to the .proto package names

mod cardano {
    include!(concat!(env!("OUT_DIR"), "/cardano.rs"));
}

#[allow(dead_code)]
mod iohk {
    pub mod jormungandr {
        include!(concat!(env!("OUT_DIR"), "/iohk.jormungandr.rs"));
    }
}

pub mod server;

// TODO: replace with network_core crate
mod network_core;
