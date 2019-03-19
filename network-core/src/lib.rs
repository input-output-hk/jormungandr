//! Abstractions for the network subsystem of a blockchain node.

#![warn(clippy::all)]

pub mod error;

pub mod client;
pub mod server;

pub mod gossip;
