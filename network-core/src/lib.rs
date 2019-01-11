//! Abstractions for the network subsystem of a blockchain node.

#[macro_use]
extern crate prost_derive;

pub mod server;

/// Common type definitions generated from protobuf.
pub mod codes {
    include!(concat!(env!("OUT_DIR"), "/iohk.chain.codes.rs"));
}
