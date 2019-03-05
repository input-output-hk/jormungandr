#![cfg_attr(feature = "with-bench", feature(test))]

#[cfg(test)]
#[cfg(feature = "with-bench")]
extern crate test;

extern crate cryptoxide;
extern crate rand_core;

mod algorithms;
mod hex;
mod kes;
mod key;
mod sign;

pub use key::{PublicKey, SecretKey};
pub use sign::Signature;

pub use algorithms::*;
