//!
//! Cardano Legacy Address generation and parsing
//!
#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

#[macro_use]
extern crate cbor_event;

extern crate cryptoxide;

extern crate ed25519_bip32;

mod base58;
mod cbor;
mod crc32;

pub mod address;
